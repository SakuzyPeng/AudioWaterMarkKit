//! ADM/BWF 保真嵌入：仅替换 data chunk，保留其他 chunk 原字节

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::audio::Audio;
use crate::error::{Error, Result};
use crate::message::MESSAGE_LEN;
use crate::multichannel::{ChannelLayout, MultichannelAudio, SampleFormat};

use super::adm_bwav::{
    parse_bed_channel_indices, parse_bed_channel_speaker_labels, parse_object_channel_indices,
    probe_adm_bwf, ChunkIndex, PcmFormat,
};
use super::adm_routing::{build_route_plan_from_labels, is_silent};

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

    // 优先：从 chna + axml 解析带 speakerLabel 的 Bed 声道列表，用于位置感知配对。
    // 失败时（axml 缺失/标签未知）显式警告，并退回按数量推断的路径。
    let bed_speaker_labels = match parse_bed_channel_speaker_labels(input, &index) {
        Ok(labels) if !labels.is_empty() => {
            // 检查是否全部标签都能被识别（无 `?AT_xxx?` 格式的未知标签）
            let all_known = labels.iter().all(|(_, l)| !l.starts_with('?'));
            if !all_known {
                let unknown: Vec<&str> = labels
                    .iter()
                    .filter(|(_, l)| l.starts_with('?'))
                    .map(|(_, l)| l.as_str())
                    .collect();
                eprintln!(
                    "[awmkit] ADM routing warning: \
                     could not resolve speaker labels for {unknown:?} via AT→AS→AC chain; \
                     falling back to channel-count-based routing"
                );
                None
            } else {
                Some(labels)
            }
        }
        Ok(_) => {
            // 空列表：无 chna 或无 Bed 声道
            None
        }
        Err(e) => {
            eprintln!(
                "[awmkit] ADM routing warning: \
                 failed to parse speaker labels from chna/axml ({e}); \
                 falling back to channel-count-based routing"
            );
            None
        }
    };

    // 退回路径：按声道索引列表（不含位置信息）
    let bed_indices = if bed_speaker_labels.is_none() {
        parse_bed_channel_indices(input, &index)?
    } else {
        None // 有 speaker_labels 时不需要 bed_indices
    };

    // 解析 Object（_0003 类型）声道索引，静默声道在嵌入时跳过
    let obj_indices = parse_object_channel_indices(input, &index).unwrap_or_default();
    if !obj_indices.is_empty() {
        eprintln!("[awmkit] ADM: found {} Object channel(s) to embed", obj_indices.len());
    }

    rewrite_adm_with_transform(input, output, &index, |source_audio| {
        // Step 1：嵌入 Bed 声道
        let mut audio = embed_adm_bed_only(
            audio_engine,
            source_audio,
            message,
            layout,
            &bed_indices,
            bed_speaker_labels.as_deref(),
        )?;
        // Step 2：嵌入 Object 声道（每个真单声道，静默跳过）
        embed_object_channels_into_audio(audio_engine, &mut audio, message, &obj_indices)?;
        Ok(audio)
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

    // 先写临时文件，替换 data chunk 后再原子 rename，避免崩溃时损坏输出。
    // 使用同目录下 PID+时间戳的随机文件名，避免与用户输出路径重合。
    let temp_output = {
        let stem = format!(
            ".awmkit_adm_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        output
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(stem)
    };
    let copy_result = fs::copy(input, &temp_output).map_err(|e| {
        Error::AdmPreserveFailed(format!(
            "failed to copy input to temp ({} -> {}): {e}",
            input.display(),
            temp_output.display()
        ))
    });
    if let Err(e) = copy_result {
        let _ = fs::remove_file(&temp_output);
        return Err(e);
    }
    if let Err(e) = replace_data_chunk_bytes(&temp_output, index, &replacement) {
        let _ = fs::remove_file(&temp_output);
        return Err(e);
    }
    fs::rename(&temp_output, output).map_err(|e| {
        let _ = fs::remove_file(&temp_output);
        Error::AdmPreserveFailed(format!(
            "failed to rename temp to output ({} -> {}): {e}",
            temp_output.display(),
            output.display()
        ))
    })
}

/// ADM Bed-only 嵌入：仅对 Bed 声道打水印，Object 声道原样保留。
///
/// - `bed_indices`: 仅声道索引（无位置信息），用于 fallback 路径。
/// - `speaker_labels`: `Some(&[(channelIndex, label)])` 时使用 axml 位置感知路由；
///   `None` 时退回按数量推断路由。两者均为 `None`/空 时走全声道路径。
fn embed_adm_bed_only(
    audio_engine: &Audio,
    source_audio: MultichannelAudio,
    message: &[u8; MESSAGE_LEN],
    layout: Option<ChannelLayout>,
    bed_indices: &Option<Vec<usize>>,
    speaker_labels: Option<&[(usize, String)]>,
) -> Result<MultichannelAudio> {
    // ── 路径 A：axml 位置感知路由 ──
    if let Some(labels) = speaker_labels {
        if !labels.is_empty() {
            return embed_bed_by_speaker_labels(audio_engine, source_audio, message, labels);
        }
    }

    // ── 路径 B：仅声道索引的 fallback 路由 ──
    let Some(indices) = bed_indices else {
        return embed_pairs_via_audiowmark(audio_engine, source_audio, message, layout);
    };
    let total_ch = source_audio.num_channels();
    if indices.is_empty() || indices.iter().any(|&i| i >= total_ch) {
        return embed_pairs_via_audiowmark(audio_engine, source_audio, message, layout);
    }
    if indices.len() == total_ch {
        return embed_pairs_via_audiowmark(audio_engine, source_audio, message, layout);
    }

    let bed_layout = layout.or_else(|| ChannelLayout::from_channels_opt(indices.len()));
    let bed_audio = extract_channels(&source_audio, indices)?;
    let watermarked_bed =
        embed_pairs_via_audiowmark(audio_engine, bed_audio, message, bed_layout)?;

    let mut result = source_audio;
    for (bed_pos, &ch_idx) in indices.iter().enumerate() {
        let samples = watermarked_bed
            .channel_samples(bed_pos)
            .map_err(|e| {
                Error::AdmPreserveFailed(format!("bed channel {bed_pos} read error: {e}"))
            })?
            .to_vec();
        result
            .replace_channel_samples(ch_idx, samples)
            .map_err(|e| {
                Error::AdmPreserveFailed(format!("bed channel {ch_idx} write error: {e}"))
            })?;
    }
    Ok(result)
}

/// 路径 A：使用 axml speakerLabel 构建 RoutePlan，按位置配对嵌入 Bed 声道。
///
/// `speaker_labels`: `(channelIndex, speakerLabel)` 列表，仅含 Bed 声道。
fn embed_bed_by_speaker_labels(
    audio_engine: &Audio,
    source_audio: MultichannelAudio,
    message: &[u8; MESSAGE_LEN],
    speaker_labels: &[(usize, String)],
) -> Result<MultichannelAudio> {
    use crate::multichannel::{effective_lfe_mode, RouteMode};

    let plan = build_route_plan_from_labels(speaker_labels, effective_lfe_mode());

    // 打印 fallback 警告（未识别标签）
    for w in &plan.warnings {
        eprintln!("[awmkit] ADM routing warning: {w}");
    }

    let total_ch = source_audio.num_channels();

    // 校验所有声道索引均在范围内
    for step in &plan.steps {
        let bad = match &step.mode {
            RouteMode::Pair(a, b) => *a >= total_ch || *b >= total_ch,
            RouteMode::Mono(a) | RouteMode::Skip { channel: a, .. } => *a >= total_ch,
        };
        if bad {
            return Err(Error::AdmPreserveFailed(format!(
                "ADM bed route step \"{}\" references channel out of range (total={})",
                step.name, total_ch
            )));
        }
    }

    let mut result = source_audio.clone();

    for step in &plan.steps {
        match &step.mode {
            RouteMode::Skip { .. } => {
                // 跳过，原样保留
            }
            RouteMode::Mono(ch) => {
                // 真单声道：单声道 WAV → audiowmark → 写回
                let mono = source_audio.channel_samples(*ch)?.to_vec();
                let mono_audio = MultichannelAudio::new(
                    vec![mono],
                    source_audio.sample_rate(),
                    source_audio.sample_format(),
                )
                .map_err(|e| {
                    Error::AdmPreserveFailed(format!("mono audio build error: {e}"))
                })?;
                match embed_single_audio_via_audiowmark(audio_engine, mono_audio, message) {
                    Ok(processed) => {
                        let samples = processed.channel_samples(0)?.to_vec();
                        result.replace_channel_samples(*ch, samples).map_err(|e| {
                            Error::AdmPreserveFailed(format!(
                                "mono ch{ch} write error: {e}"
                            ))
                        })?;
                    }
                    Err(e) => {
                        eprintln!(
                            "[awmkit] ADM routing warning: \
                             step \"{}\" (Mono ch{ch}) failed: {e}; channel unchanged",
                            step.name
                        );
                    }
                }
            }
            RouteMode::Pair(ch_l, ch_r) => {
                let left  = source_audio.channel_samples(*ch_l)?.to_vec();
                let right = source_audio.channel_samples(*ch_r)?.to_vec();
                let stereo = MultichannelAudio::new(
                    vec![left, right],
                    source_audio.sample_rate(),
                    source_audio.sample_format(),
                )
                .map_err(|e| {
                    Error::AdmPreserveFailed(format!("stereo audio build error: {e}"))
                })?;
                match embed_single_audio_via_audiowmark(audio_engine, stereo, message) {
                    Ok(processed) => {
                        let l_samples = processed.channel_samples(0)?.to_vec();
                        let r_samples = processed.channel_samples(1)?.to_vec();
                        result.replace_channel_samples(*ch_l, l_samples).map_err(|e| {
                            Error::AdmPreserveFailed(format!(
                                "pair ch{ch_l} write error: {e}"
                            ))
                        })?;
                        result.replace_channel_samples(*ch_r, r_samples).map_err(|e| {
                            Error::AdmPreserveFailed(format!(
                                "pair ch{ch_r} write error: {e}"
                            ))
                        })?;
                    }
                    Err(e) => {
                        eprintln!(
                            "[awmkit] ADM routing warning: \
                             step \"{}\" (Pair ch{ch_l}+ch{ch_r}) failed: {e}; channels unchanged",
                            step.name
                        );
                    }
                }
            }
        }
    }

    Ok(result)
}

/// 对 ADM Object 声道列表依序嵌入水印（真单声道，各自独立）。
///
/// 处理流程（对每个 `obj_idx`）：
/// 1. 越界检查 → 跳过并警告
/// 2. 静默检测（~-80 dBFS）→ 跳过并提示
/// 3. 构建 1ch [`MultichannelAudio`] → audiowmark → 写回原声道
/// 4. 失败 → 警告（原声道不变，不影响整体流程）
fn embed_object_channels_into_audio(
    audio_engine: &Audio,
    audio: &mut MultichannelAudio,
    message: &[u8; MESSAGE_LEN],
    obj_indices: &[usize],
) -> Result<()> {
    let total_ch = audio.num_channels();
    let sf = audio.sample_format();

    for &obj_idx in obj_indices {
        if obj_idx >= total_ch {
            eprintln!(
                "[awmkit] ADM: Object ch{obj_idx} out of range (total={total_ch}), skipping"
            );
            continue;
        }
        let samples = audio
            .channel_samples(obj_idx)
            .map_err(|e| {
                Error::AdmPreserveFailed(format!("Object ch{obj_idx} channel_samples error: {e}"))
            })?;
        if is_silent(samples, sf) {
            eprintln!("[awmkit] ADM: Object ch{obj_idx} is silent (~-80 dBFS), skipping");
            continue;
        }
        let mono_audio = MultichannelAudio::new(
            vec![samples.to_vec()],
            audio.sample_rate(),
            sf,
        )
        .map_err(|e| {
            Error::AdmPreserveFailed(format!("Object ch{obj_idx} mono build error: {e}"))
        })?;
        match embed_single_audio_via_audiowmark(audio_engine, mono_audio, message) {
            Ok(processed) => {
                let new_samples = processed
                    .channel_samples(0)
                    .map_err(|e| {
                        Error::AdmPreserveFailed(format!(
                            "Object ch{obj_idx} result channel_samples error: {e}"
                        ))
                    })?
                    .to_vec();
                audio.replace_channel_samples(obj_idx, new_samples).map_err(|e| {
                    Error::AdmPreserveFailed(format!("Object ch{obj_idx} write-back error: {e}"))
                })?;
            }
            Err(e) => {
                eprintln!(
                    "[awmkit] ADM routing warning: Object ch{obj_idx} embed failed: {e}; \
                     channel unchanged"
                );
            }
        }
    }
    Ok(())
}

/// 对单个 [`MultichannelAudio`]（1ch mono 或 2ch stereo）执行 audiowmark 嵌入。
///
/// 写临时文件 → audiowmark → 读回。
fn embed_single_audio_via_audiowmark(
    audio_engine: &Audio,
    audio: MultichannelAudio,
    message: &[u8; MESSAGE_LEN],
) -> Result<MultichannelAudio> {
    let temp_dir = create_temp_dir("awmkit_adm_step")?;
    let temp_in  = temp_dir.join("step_in.wav");
    let temp_out = temp_dir.join("step_out.wav");

    let result = (|| {
        audio.to_wav(&temp_in)?;
        audio_engine.embed_multichannel(
            &temp_in,
            &temp_out,
            message,
            Some(audio.layout()),
        )?;
        let bytes = fs::read(&temp_out).map_err(|e| {
            Error::AdmPreserveFailed(format!("failed to read step output: {e}"))
        })?;
        MultichannelAudio::from_wav_bytes(&bytes)
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

/// 从 MultichannelAudio 按索引提取子集声道（供外部模块调用）。
pub(crate) fn extract_bed_channels(
    audio: &MultichannelAudio,
    indices: &[usize],
) -> Result<MultichannelAudio> {
    extract_channels(audio, indices)
}

/// 从 MultichannelAudio 按索引提取子集声道。
fn extract_channels(audio: &MultichannelAudio, indices: &[usize]) -> Result<MultichannelAudio> {
    let mut channels = Vec::with_capacity(indices.len());
    for &idx in indices {
        channels.push(audio.channel_samples(idx)?.to_vec());
    }
    MultichannelAudio::new(channels, audio.sample_rate(), audio.sample_format())
        .map_err(|e| Error::AdmPreserveFailed(format!("failed to build bed audio: {e}")))
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

    let _ = fs::remove_dir_all(&temp_dir);
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

pub(crate) fn decode_pcm_audio(path: &Path, index: &ChunkIndex) -> Result<MultichannelAudio> {
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

    for frame in 0..frame_count {
        for ch in 0..expected_channels {
            let sample = audio
                .channel_samples(ch)
                .map_err(|e| Error::AdmPreserveFailed(format!("channel {ch} access error: {e}")))?
                .get(frame)
                .copied()
                .ok_or_else(|| {
                    Error::AdmPreserveFailed(format!(
                        "channel {ch} sample index {frame} out of range"
                    ))
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
