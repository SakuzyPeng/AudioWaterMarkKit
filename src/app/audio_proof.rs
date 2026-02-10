use crate::app::error::{AppError, Result};
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

pub fn build_audio_proof<P: AsRef<Path>>(path: P) -> Result<AudioProof> {
    let audio = MultichannelAudio::from_file(path)?;
    let sample_rate = audio.sample_rate();
    let channels = u32::try_from(audio.num_channels())
        .map_err(|_| AppError::Message("channel count overflow".to_string()))?;
    let sample_count = u64::try_from(audio.num_samples())
        .map_err(|_| AppError::Message("sample count overflow".to_string()))?;
    let sample_format = audio.sample_format();
    let interleaved = audio.interleaved_samples();

    let pcm_sha256 = pcm_sha256_for_interleaved(sample_rate, channels, sample_count, &interleaved);
    let samples_i16 = to_i16_samples(&interleaved, sample_format);

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

    let chunk = 16_384;
    for data in samples_i16.chunks(chunk) {
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

fn to_i16_samples(samples: &[i32], sample_format: SampleFormat) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| sample_to_i16(*sample, sample_format))
        .collect()
}

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
}
