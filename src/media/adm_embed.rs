//! ADM/BWF 保真嵌入：仅替换 data chunk，保留其他 chunk 原字节

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::audio::Audio;
use crate::error::{Error, Result};
use crate::message::MESSAGE_LEN;
use crate::multichannel::{ChannelLayout, MultichannelAudio, SampleFormat};

use super::adm_bwav::{probe_adm_bwf, ChunkIndex, PcmFormat};

pub(crate) fn embed_adm_multichannel(
    audio_engine: &Audio,
    input: &Path,
    output: &Path,
    message: &[u8; MESSAGE_LEN],
    layout: Option<ChannelLayout>,
) -> Result<()> {
    if input == output {
        return Err(Error::AdmPreserveFailed(
            "input and output must be different files for ADM/BWF embed".to_string(),
        ));
    }

    let Some(index) = probe_adm_bwf(input)? else {
        return Err(Error::AdmUnsupported(format!(
            "ADM/BWF metadata not detected: {}",
            input.display()
        )));
    };

    rewrite_adm_with_transform(input, output, &index, |source_audio| {
        embed_pairs_via_audiowmark(audio_engine, source_audio, message, layout)
    })
}

pub(crate) fn rewrite_adm_with_transform<F>(
    input: &Path,
    output: &Path,
    index: &ChunkIndex,
    mut transform: F,
) -> Result<()>
where
    F: FnMut(MultichannelAudio) -> Result<MultichannelAudio>,
{
    let original_audio = decode_pcm_audio(input, index)?;
    let processed_audio = transform(original_audio.clone())?;
    validate_audio_shape(&original_audio, &processed_audio)?;

    let replacement = encode_pcm_audio_data(&processed_audio, index.fmt)?;
    let expected_size = usize::try_from(index.data_chunk.size)
        .map_err(|_| Error::AdmPreserveFailed("data chunk too large to encode".to_string()))?;
    if replacement.len() != expected_size {
        return Err(Error::AdmPreserveFailed(format!(
            "processed data size mismatch: expected {expected_size}, got {}",
            replacement.len()
        )));
    }

    fs::copy(input, output).map_err(|e| {
        Error::AdmPreserveFailed(format!(
            "failed to copy input to output ({} -> {}): {e}",
            input.display(),
            output.display()
        ))
    })?;
    replace_data_chunk_bytes(output, index, &replacement)?;
    Ok(())
}

fn embed_pairs_via_audiowmark(
    audio_engine: &Audio,
    source_audio: MultichannelAudio,
    message: &[u8; MESSAGE_LEN],
    layout: Option<ChannelLayout>,
) -> Result<MultichannelAudio> {
    let selected_layout = layout.unwrap_or_else(|| source_audio.layout());
    let selected_channels = usize::from(selected_layout.channels());
    if selected_channels != source_audio.num_channels() {
        return Err(Error::AdmUnsupported(format!(
            "channel layout mismatch: layout={}ch, source={}ch",
            selected_channels,
            source_audio.num_channels()
        )));
    }

    let temp_dir = create_temp_dir("awmkit_adm_embed")?;
    let temp_input = temp_dir.join("source_multichannel.wav");
    let temp_output = temp_dir.join("embedded_multichannel.wav");

    let embed_result = (|| {
        source_audio.to_wav(&temp_input)?;
        audio_engine.embed_multichannel(
            &temp_input,
            &temp_output,
            message,
            Some(selected_layout),
        )?;
        // embed_multichannel 在 pipe 模式下输出 RIFF ffffffff 格式；
        // 通过 from_wav_bytes 读取可自动修复大小字段。
        let temp_bytes = fs::read(&temp_output).map_err(|e| {
            crate::error::Error::AdmPreserveFailed(format!(
                "failed to read embedded temp output: {e}"
            ))
        })?;
        MultichannelAudio::from_wav_bytes(&temp_bytes)
    })();

    let _ = fs::remove_file(&temp_input);
    let _ = fs::remove_file(&temp_output);
    let _ = fs::remove_dir(&temp_dir);
    embed_result
}

fn validate_audio_shape(original: &MultichannelAudio, processed: &MultichannelAudio) -> Result<()> {
    if original.num_channels() != processed.num_channels() {
        return Err(Error::AdmPreserveFailed(format!(
            "channel count changed after transform: {} -> {}",
            original.num_channels(),
            processed.num_channels()
        )));
    }
    if original.num_samples() != processed.num_samples() {
        return Err(Error::AdmPreserveFailed(format!(
            "sample count changed after transform: {} -> {}",
            original.num_samples(),
            processed.num_samples()
        )));
    }
    if original.sample_rate() != processed.sample_rate() {
        return Err(Error::AdmPreserveFailed(format!(
            "sample rate changed after transform: {} -> {}",
            original.sample_rate(),
            processed.sample_rate()
        )));
    }
    if original.sample_format() != processed.sample_format() {
        return Err(Error::AdmPreserveFailed(format!(
            "sample format changed after transform: {:?} -> {:?}",
            original.sample_format(),
            processed.sample_format()
        )));
    }
    Ok(())
}

fn decode_pcm_audio(path: &Path, index: &ChunkIndex) -> Result<MultichannelAudio> {
    let data_size = usize::try_from(index.data_chunk.size)
        .map_err(|_| Error::AdmUnsupported("data chunk too large to decode".to_string()))?;
    let mut file = File::open(path).map_err(|e| {
        Error::AdmUnsupported(format!(
            "failed to open {} for PCM read: {e}",
            path.display()
        ))
    })?;
    file.seek(SeekFrom::Start(index.data_chunk.data_offset))
        .map_err(|e| Error::AdmUnsupported(format!("failed to seek data chunk: {e}")))?;

    let mut data = vec![0_u8; data_size];
    file.read_exact(&mut data)
        .map_err(|e| Error::AdmUnsupported(format!("failed to read data chunk: {e}")))?;

    let channels = usize::from(index.fmt.channels);
    let frame_size = usize::from(index.fmt.block_align);
    if frame_size == 0 {
        return Err(Error::AdmUnsupported(
            "block_align cannot be zero".to_string(),
        ));
    }
    if data.len() % frame_size != 0 {
        return Err(Error::AdmUnsupported(format!(
            "data payload size {} is not aligned to frame size {}",
            data.len(),
            frame_size
        )));
    }

    let frame_count = data.len() / frame_size;
    let mut separated = vec![Vec::with_capacity(frame_count); channels];
    let sample_width = usize::from(index.fmt.bytes_per_sample);

    for frame in 0..frame_count {
        let frame_base = frame
            .checked_mul(frame_size)
            .ok_or_else(|| Error::AdmUnsupported("frame offset overflow".to_string()))?;
        for (ch, channel) in separated.iter_mut().enumerate().take(channels) {
            let sample_base =
                frame_base
                    .checked_add(ch.checked_mul(sample_width).ok_or_else(|| {
                        Error::AdmUnsupported("sample offset overflow".to_string())
                    })?)
                    .ok_or_else(|| Error::AdmUnsupported("sample offset overflow".to_string()))?;
            let sample = decode_sample(&data, sample_base, index.fmt.bits_per_sample)?;
            channel.push(sample);
        }
    }

    let sample_format = sample_format_from_bits(index.fmt.bits_per_sample)?;
    MultichannelAudio::new(separated, index.fmt.sample_rate, sample_format)
}

fn encode_pcm_audio_data(audio: &MultichannelAudio, fmt: PcmFormat) -> Result<Vec<u8>> {
    let channels = audio.num_channels();
    let expected_channels = usize::from(fmt.channels);
    if channels != expected_channels {
        return Err(Error::AdmPreserveFailed(format!(
            "channel count mismatch for PCM encode: {} != {}",
            channels, expected_channels
        )));
    }

    let sample_bits = fmt.bits_per_sample;
    let sample_width = usize::from(fmt.bytes_per_sample);
    let frame_size = usize::from(fmt.block_align);
    let frame_count = audio.num_samples();
    let mut out = vec![
        0_u8;
        frame_count.checked_mul(frame_size).ok_or_else(|| {
            Error::AdmPreserveFailed("encoded PCM size overflow".to_string())
        })?
    ];

    let pairs = audio.split_stereo_pairs();
    let mut channels_data = Vec::with_capacity(expected_channels);
    for (left, right) in pairs {
        channels_data.push(left);
        channels_data.push(right);
    }
    if channels_data.len() != expected_channels {
        return Err(Error::AdmPreserveFailed(format!(
            "unexpected channel split result: {} != {}",
            channels_data.len(),
            expected_channels
        )));
    }

    for frame in 0..frame_count {
        for (ch, ch_data) in channels_data.iter().enumerate().take(expected_channels) {
            let sample = ch_data.get(frame).copied().ok_or_else(|| {
                Error::AdmPreserveFailed("channel sample index out of range".to_string())
            })?;
            let base = frame
                .checked_mul(frame_size)
                .and_then(|v| v.checked_add(ch.checked_mul(sample_width)?))
                .ok_or_else(|| {
                    Error::AdmPreserveFailed("encoded PCM offset overflow".to_string())
                })?;
            encode_sample(&mut out, base, sample_bits, sample)?;
        }
    }

    Ok(out)
}

fn decode_sample(data: &[u8], base: usize, bits: u16) -> Result<i32> {
    match bits {
        16 => {
            let end = base
                .checked_add(2)
                .ok_or_else(|| Error::AdmUnsupported("sample slice overflow".to_string()))?;
            let bytes = data
                .get(base..end)
                .ok_or_else(|| Error::AdmUnsupported("sample slice out of range".to_string()))?;
            Ok(i32::from(i16::from_le_bytes([bytes[0], bytes[1]])))
        }
        24 => {
            let end = base
                .checked_add(3)
                .ok_or_else(|| Error::AdmUnsupported("sample slice overflow".to_string()))?;
            let bytes = data
                .get(base..end)
                .ok_or_else(|| Error::AdmUnsupported("sample slice out of range".to_string()))?;
            let sign = if (bytes[2] & 0x80) != 0 { 0xFF } else { 0x00 };
            Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], sign]))
        }
        32 => {
            let end = base
                .checked_add(4)
                .ok_or_else(|| Error::AdmUnsupported("sample slice overflow".to_string()))?;
            let bytes = data
                .get(base..end)
                .ok_or_else(|| Error::AdmUnsupported("sample slice out of range".to_string()))?;
            Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        _ => Err(Error::AdmPcmFormatUnsupported(format!(
            "unsupported bit depth for decode: {bits}"
        ))),
    }
}

fn encode_sample(data: &mut [u8], base: usize, bits: u16, sample: i32) -> Result<()> {
    match bits {
        16 => {
            let clamped = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
            let out = i16::try_from(clamped).map_err(|_| {
                Error::AdmPreserveFailed(format!("failed to cast i16 sample: {clamped}"))
            })?;
            let bytes = out.to_le_bytes();
            write_sample_bytes(data, base, &bytes)
        }
        24 => {
            let clamped = sample.clamp(-8_388_608, 8_388_607);
            let bytes = [
                (clamped & 0xFF) as u8,
                ((clamped >> 8) & 0xFF) as u8,
                ((clamped >> 16) & 0xFF) as u8,
            ];
            write_sample_bytes(data, base, &bytes)
        }
        32 => write_sample_bytes(data, base, &sample.to_le_bytes()),
        _ => Err(Error::AdmPcmFormatUnsupported(format!(
            "unsupported bit depth for encode: {bits}"
        ))),
    }
}

fn write_sample_bytes(data: &mut [u8], base: usize, bytes: &[u8]) -> Result<()> {
    let end = base
        .checked_add(bytes.len())
        .ok_or_else(|| Error::AdmPreserveFailed("sample write overflow".to_string()))?;
    let dst = data
        .get_mut(base..end)
        .ok_or_else(|| Error::AdmPreserveFailed("sample write out of range".to_string()))?;
    dst.copy_from_slice(bytes);
    Ok(())
}

fn replace_data_chunk_bytes(path: &Path, index: &ChunkIndex, replacement: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| Error::AdmPreserveFailed(format!("failed to open output file: {e}")))?;
    file.seek(SeekFrom::Start(index.data_chunk.data_offset))
        .map_err(|e| Error::AdmPreserveFailed(format!("failed to seek output data chunk: {e}")))?;
    file.write_all(replacement)
        .map_err(|e| Error::AdmPreserveFailed(format!("failed to write output data chunk: {e}")))?;
    file.flush()
        .map_err(|e| Error::AdmPreserveFailed(format!("failed to flush output data chunk: {e}")))?;
    Ok(())
}

fn sample_format_from_bits(bits: u16) -> Result<SampleFormat> {
    match bits {
        16 => Ok(SampleFormat::Int16),
        24 => Ok(SampleFormat::Int24),
        32 => Ok(SampleFormat::Int32),
        _ => Err(Error::AdmPcmFormatUnsupported(format!(
            "unsupported bit depth: {bits}"
        ))),
    }
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "{prefix}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn push_chunk(dst: &mut Vec<u8>, id: [u8; 4], payload: &[u8]) {
        let size = u32::try_from(payload.len()).unwrap_or(0);
        dst.extend_from_slice(&id);
        dst.extend_from_slice(&size.to_le_bytes());
        dst.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            dst.push(0);
        }
    }

    fn build_adm_riff_wave() -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        fmt.extend_from_slice(&2_u16.to_le_bytes()); // ch
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&(48_000_u32 * 2 * 3).to_le_bytes());
        fmt.extend_from_slice(&6_u16.to_le_bytes()); // block align
        fmt.extend_from_slice(&24_u16.to_le_bytes()); // bits

        let data_payload = vec![
            1, 0, 0, 2, 0, 0, // frame0
            3, 0, 0, 4, 0, 0, // frame1
            5, 0, 0, 6, 0, 0, // frame2
        ];

        let mut chunks = Vec::new();
        push_chunk(&mut chunks, *b"fmt ", &fmt);
        push_chunk(&mut chunks, *b"bext", b"bextv1");
        push_chunk(&mut chunks, *b"axml", b"<adm/>");
        push_chunk(&mut chunks, *b"chna", &[1, 0, 0, 0]);
        push_chunk(&mut chunks, *b"zzzz", &[9, 8, 7, 6, 5]);
        push_chunk(&mut chunks, *b"data", &data_payload);

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        let riff_size = u32::try_from(chunks.len() + 4).unwrap_or(0);
        out.extend_from_slice(&riff_size.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(&chunks);
        out
    }

    fn write_temp_file(prefix: &str, data: &[u8]) -> Result<PathBuf> {
        let path = std::env::temp_dir().join(format!(
            "{prefix}_{}_{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::write(&path, data)?;
        Ok(path)
    }

    #[test]
    fn rewrite_preserves_non_audio_chunks() {
        let input_bytes = build_adm_riff_wave();
        let input_path_result = write_temp_file("adm_embed_in", &input_bytes);
        assert!(input_path_result.is_ok());
        let Ok(input_path) = input_path_result else {
            return;
        };
        let output_path = input_path.with_file_name("adm_embed_out.wav");

        let parsed = super::super::adm_bwav::parse_chunk_index(&input_path);
        assert!(parsed.is_ok());
        let Ok(parsed_opt) = parsed else {
            let _ = fs::remove_file(&input_path);
            return;
        };
        assert!(parsed_opt.is_some());
        let Some(index) = parsed_opt else {
            let _ = fs::remove_file(&input_path);
            return;
        };

        let rewritten = rewrite_adm_with_transform(&input_path, &output_path, &index, |audio| {
            let mut pairs = audio.split_stereo_pairs();
            if let Some((left, _right)) = pairs.first_mut() {
                if let Some(first_sample) = left.first_mut() {
                    *first_sample += 111;
                }
            }
            MultichannelAudio::merge_stereo_pairs(
                &pairs,
                audio.sample_rate(),
                audio.sample_format(),
            )
        });
        assert!(rewritten.is_ok());

        let before = fs::read(&input_path);
        let after = fs::read(&output_path);
        assert!(before.is_ok() && after.is_ok());
        let Ok(mut before_bytes) = before else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };
        let Ok(mut after_bytes) = after else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };

        let size = usize::try_from(index.data_chunk.size).unwrap_or(0);
        let offset = usize::try_from(index.data_chunk.data_offset).unwrap_or(0);
        if size > 0 && offset > 0 && offset.saturating_add(size) <= before_bytes.len() {
            before_bytes[offset..offset + size].fill(0);
            after_bytes[offset..offset + size].fill(0);
        }
        assert_eq!(before_bytes, after_bytes);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&output_path);
    }

    #[test]
    fn rewrite_changes_data_keeps_format() {
        let input_bytes = build_adm_riff_wave();
        let input_path_result = write_temp_file("adm_embed_fmt_in", &input_bytes);
        assert!(input_path_result.is_ok());
        let Ok(input_path) = input_path_result else {
            return;
        };
        let output_path = input_path.with_file_name("adm_embed_fmt_out.wav");

        let parsed = super::super::adm_bwav::parse_chunk_index(&input_path);
        assert!(parsed.is_ok());
        let Ok(parsed_opt) = parsed else {
            let _ = fs::remove_file(&input_path);
            return;
        };
        let Some(index) = parsed_opt else {
            let _ = fs::remove_file(&input_path);
            return;
        };

        let rewritten = rewrite_adm_with_transform(&input_path, &output_path, &index, |audio| {
            let mut pairs = audio.split_stereo_pairs();
            if let Some((left, right)) = pairs.first_mut() {
                if let Some(sample) = left.first_mut() {
                    *sample += 200;
                }
                if let Some(sample) = right.first_mut() {
                    *sample += 200;
                }
            }
            MultichannelAudio::merge_stereo_pairs(
                &pairs,
                audio.sample_rate(),
                audio.sample_format(),
            )
        });
        assert!(rewritten.is_ok());

        let out_parsed = super::super::adm_bwav::parse_chunk_index(&output_path);
        assert!(out_parsed.is_ok());
        let Ok(out_opt) = out_parsed else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };
        let Some(out_index) = out_opt else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };
        assert_eq!(index.fmt.channels, out_index.fmt.channels);
        assert_eq!(index.fmt.bits_per_sample, out_index.fmt.bits_per_sample);
        assert_eq!(index.fmt.sample_rate, out_index.fmt.sample_rate);

        let in_audio = decode_pcm_audio(&input_path, &index);
        let out_audio = decode_pcm_audio(&output_path, &out_index);
        assert!(in_audio.is_ok() && out_audio.is_ok());
        let Ok(in_audio) = in_audio else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };
        let Ok(out_audio) = out_audio else {
            let _ = fs::remove_file(&input_path);
            let _ = fs::remove_file(&output_path);
            return;
        };

        let in_pairs = in_audio.split_stereo_pairs();
        let out_pairs = out_audio.split_stereo_pairs();
        assert!(!in_pairs.is_empty() && !out_pairs.is_empty());
        assert_ne!(in_pairs[0].0[0], out_pairs[0].0[0]);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&output_path);
    }
}
