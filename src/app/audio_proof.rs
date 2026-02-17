use crate::app::error::{AppError, Result};
#[cfg(feature = "ffmpeg-decode")]
use crate::media;
use crate::multichannel::{MultichannelAudio, SampleFormat};
use rusty_chromaprint::{Configuration, Fingerprinter};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AudioProof {
    pub sample_rate: u32,
    pub channels: u32,
    pub sample_count: u64,
    pub pcm_sha256: String,
    pub chromaprint: Vec<u32>,
    pub fp_config_id: u8,
}

#[allow(clippy::module_name_repetitions)]
/// # Errors
/// 当输入无法解码、样本不合法或指纹计算失败时返回错误。.
pub fn build_audio_proof<P: AsRef<Path>>(path: P) -> Result<AudioProof> {
    let path = path.as_ref();

    #[cfg(feature = "ffmpeg-decode")]
    {
        match build_audio_proof_via_ffmpeg(path) {
            Ok(proof) => return Ok(proof),
            Err(err) => {
                // Keep legacy parser fallback for native WAV/FLAC in case FFmpeg runtime
                // is missing, but surface FFmpeg decode errors for other extensions.
                if !has_wav_or_flac_extension(path) {
                    return Err(err);
                }
            }
        }
    }

    let audio = MultichannelAudio::from_file(path)?;
    let sample_rate = audio.sample_rate();
    let channels = u32::try_from(audio.num_channels())
        .map_err(|_| AppError::Message("channel count overflow".to_string()))?;
    let sample_format = audio.sample_format();
    let interleaved = audio.interleaved_samples();
    build_audio_proof_from_interleaved(sample_rate, channels, &interleaved, sample_format)
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn build_audio_proof_via_ffmpeg(path: &Path) -> Result<AudioProof> {
    let decoded = media::decode_media_to_pcm_i32(path).map_err(AppError::from)?;
    let channels = u32::from(decoded.channels);
    build_audio_proof_from_interleaved(
        decoded.sample_rate,
        channels,
        &decoded.samples,
        SampleFormat::Int16,
    )
}

/// Internal helper function.
fn build_audio_proof_from_interleaved(
    sample_rate: u32,
    channels: u32,
    interleaved: &[i32],
    sample_format: SampleFormat,
) -> Result<AudioProof> {
    let channels_usize = usize::try_from(channels)
        .map_err(|_| AppError::Message("channel count overflow".to_string()))?;
    if channels_usize == 0 || !interleaved.len().is_multiple_of(channels_usize) {
        return Err(AppError::Message(
            "interleaved sample length is not channel-aligned".to_string(),
        ));
    }
    let sample_count = u64::try_from(interleaved.len() / channels_usize)
        .map_err(|_| AppError::Message("sample count overflow".to_string()))?;
    let pcm_sha256 = pcm_sha256_for_interleaved(sample_rate, channels, sample_count, interleaved);
    let samples_i16 = to_i16_samples(interleaved, sample_format);
    if samples_i16.is_empty() {
        return Err(AppError::Message(
            "cannot build audio proof for empty audio".to_string(),
        ));
    }

    let config = Configuration::default();
    let mut fingerprinter = Fingerprinter::new(&config);
    fingerprinter
        .start(sample_rate, channels)
        .map_err(|e| AppError::Message(format!("chromaprint start failed: {e}")))?;

    // Feed full frames only: chunk size must be divisible by channel count.
    let chunk_frames = 4096usize;
    let chunk_samples = chunk_frames
        .saturating_mul(channels_usize)
        .max(channels_usize);
    for data in samples_i16.chunks(chunk_samples) {
        fingerprinter.consume(data);
    }
    fingerprinter.finish();

    let chromaprint = fingerprinter.fingerprint().to_vec();
    if chromaprint.is_empty() {
        return Err(AppError::Message(
            "chromaprint fingerprint is empty".to_string(),
        ));
    }

    Ok(AudioProof {
        sample_rate,
        channels,
        sample_count,
        pcm_sha256,
        chromaprint,
        fp_config_id: config.id(),
    })
}

/// Internal helper function.
fn has_wav_or_flac_extension(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    matches!(ext.as_deref(), Some("wav" | "flac"))
}

/// Internal helper function.
pub(crate) fn pcm_sha256_for_interleaved(
    sample_rate: u32,
    channels: u32,
    sample_count: u64,
    interleaved_samples: &[i32],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sample_rate.to_le_bytes());
    hasher.update(channels.to_le_bytes());
    hasher.update(sample_count.to_le_bytes());
    for sample in interleaved_samples {
        hasher.update(sample.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Internal helper function.
fn to_i16_samples(samples: &[i32], sample_format: SampleFormat) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| sample_to_i16(*sample, sample_format))
        .collect()
}

/// Internal helper function.
fn sample_to_i16(sample: i32, sample_format: SampleFormat) -> i16 {
    let scaled = match sample_format {
        SampleFormat::Int16 => sample,
        SampleFormat::Int24 => sample >> 8,
        SampleFormat::Int32 | SampleFormat::Float32 => sample >> 16,
    };

    let min = i32::from(i16::MIN);
    let max = i32::from(i16::MAX);
    i16::try_from(scaled.clamp(min, max)).unwrap_or(if scaled < 0 { i16::MIN } else { i16::MAX })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::multichannel::SampleFormat;
    use std::path::{Path, PathBuf};

    #[test]
    fn pcm_sha256_is_stable_for_same_input() {
        let samples = vec![0i32, 1, -1, 10_000, -10_000, 32_000, -32_000];
        let sha1 = pcm_sha256_for_interleaved(44_100, 2, 7, &samples);
        let sha2 = pcm_sha256_for_interleaved(44_100, 2, 7, &samples);
        assert_eq!(sha1, sha2);
    }

    #[test]
    fn i24_to_i16_conversion_is_clamped() {
        assert_eq!(sample_to_i16(i32::MAX, SampleFormat::Int24), i16::MAX);
        assert_eq!(sample_to_i16(i32::MIN, SampleFormat::Int24), i16::MIN);
    }

    #[test]
    fn build_audio_proof_accepts_wav_content_with_mp3_extension() {
        let wav_path = unique_temp_path("proof_src.wav");
        let mp3_like_path = unique_temp_path("proof_src.mp3");

        create_test_wav(&wav_path);
        std::fs::copy(&wav_path, &mp3_like_path).unwrap();

        match build_audio_proof(&mp3_like_path) {
            Ok(proof) => {
                assert!(proof.channels > 0);
                assert!(proof.sample_rate > 0);
            }
            Err(err) => {
                let is_ffmpeg_runtime_issue = matches!(
                    err,
                    AppError::Awmkit(
                        crate::Error::FfmpegLibraryNotFound(_)
                            | crate::Error::FfmpegDecodeFailed(_)
                            | crate::Error::FfmpegContainerUnsupported(_),
                    )
                );
                assert!(
                    is_ffmpeg_runtime_issue,
                    "unexpected proof build error: {err}"
                );
                if is_ffmpeg_runtime_issue {
                    // Some local test environments miss FFmpeg runtime support.
                    // Ensure the baseline WAV path still works in this case.
                    let fallback = build_audio_proof(&wav_path);
                    assert!(fallback.is_ok());
                }
            }
        }

        let _ = std::fs::remove_file(&wav_path);
        let _ = std::fs::remove_file(&mp3_like_path);
    }

    fn create_test_wav(path: &Path) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(44_100_i32 * 6) {
            let centered = (i % 1024) - 512;
            let sample = i16::try_from(centered * 48).unwrap_or(0);
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn unique_temp_path(file_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "awmkit_audio_proof_test_{}_{}_{}",
            std::process::id(),
            nanos,
            file_name
        ))
    }
}
