//! FFmpeg 动态库解码后端

use std::ffi::CString;
use std::path::Path;
use std::sync::OnceLock;

use ffmpeg_next as ffmpeg;

use crate::audio::{AudioMediaCapabilities, DecodedPcm};
use crate::error::{Error, Result};

static FFMPEG_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

pub(crate) fn decode_media_to_pcm_i32(input: &Path) -> Result<DecodedPcm> {
    ensure_ffmpeg_initialized()?;

    let mut input_ctx =
        ffmpeg::format::input(input).map_err(|err| map_open_error(input, &err.to_string()))?;

    let stream = input_ctx
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or_else(|| Error::InvalidInput("no decodable audio track found".to_string()))?;
    let stream_index = stream.index();
    let stream_codec_id = stream.parameters().id();

    if stream_codec_id == ffmpeg::codec::Id::EAC3
        && ffmpeg::codec::decoder::find(ffmpeg::codec::Id::EAC3).is_none()
    {
        return Err(Error::FfmpegDecoderUnavailable("eac3".to_string()));
    }

    let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|err| Error::FfmpegDecodeFailed(format!("failed to load codec context: {err}")))?;
    let mut decoder = context
        .decoder()
        .audio()
        .map_err(|err| Error::FfmpegDecodeFailed(format!("failed to open audio decoder: {err}")))?;

    let sample_rate = decoder.rate();
    let channels = decoder.channels();
    if channels == 0 || sample_rate == 0 {
        return Err(Error::FfmpegDecodeFailed(
            "decoded audio metadata is invalid".to_string(),
        ));
    }

    let output_layout = normalize_layout(decoder.channel_layout(), channels);
    let output_rate = sample_rate;
    let mut resampler = create_resampler(
        decoder.format(),
        output_layout,
        sample_rate,
        output_layout,
        output_rate,
    )?;

    let mut decoded = ffmpeg::frame::Audio::empty();
    let mut samples = Vec::<i32>::new();

    for (packet_stream, packet) in input_ctx.packets() {
        if packet_stream.index() != stream_index {
            continue;
        }

        decoder.send_packet(&packet).map_err(|err| {
            Error::FfmpegDecodeFailed(format!("decoder send packet failed: {err}"))
        })?;
        receive_decoded_frames(
            &mut decoder,
            &mut resampler,
            &mut decoded,
            &mut samples,
            output_layout,
            output_rate,
        )?;
    }

    decoder
        .send_eof()
        .map_err(|err| Error::FfmpegDecodeFailed(format!("decoder send eof failed: {err}")))?;
    receive_decoded_frames(
        &mut decoder,
        &mut resampler,
        &mut decoded,
        &mut samples,
        output_layout,
        output_rate,
    )?;
    flush_resampler(&mut resampler, &mut samples)?;

    if samples.is_empty() {
        return Err(Error::FfmpegDecodeFailed(
            "no decodable audio samples found".to_string(),
        ));
    }

    Ok(DecodedPcm {
        sample_rate,
        channels,
        bits_per_sample: 16,
        samples,
    })
}

pub(crate) fn media_capabilities() -> AudioMediaCapabilities {
    if ensure_ffmpeg_initialized().is_err() {
        return AudioMediaCapabilities {
            backend: "ffmpeg",
            eac3_decode: false,
            container_mp4: false,
            container_mkv: false,
            container_ts: false,
        };
    }

    AudioMediaCapabilities {
        backend: "ffmpeg",
        eac3_decode: ffmpeg::codec::decoder::find(ffmpeg::codec::Id::EAC3).is_some(),
        container_mp4: has_demuxer("mov"),
        container_mkv: has_demuxer("matroska"),
        container_ts: has_demuxer("mpegts"),
    }
}

fn ensure_ffmpeg_initialized() -> Result<()> {
    match FFMPEG_INIT.get_or_init(|| ffmpeg::init().map_err(|err| err.to_string())) {
        Ok(()) => Ok(()),
        Err(err) => Err(Error::FfmpegLibraryNotFound(err.clone())),
    }
}

fn map_open_error(input: &Path, detail: &str) -> Error {
    let ext = input
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if matches!(ext.as_str(), "ts" | "m2ts" | "m2t") && !has_demuxer("mpegts") {
        return Error::FfmpegContainerUnsupported("mpegts".to_string());
    }
    if matches!(ext.as_str(), "mkv" | "mka") && !has_demuxer("matroska") {
        return Error::FfmpegContainerUnsupported("matroska".to_string());
    }
    if matches!(ext.as_str(), "mp4" | "m4a" | "mov") && !has_demuxer("mov") {
        return Error::FfmpegContainerUnsupported("mov/mp4".to_string());
    }

    Error::FfmpegDecodeFailed(format!("failed to open input media: {detail}"))
}

fn receive_decoded_frames(
    decoder: &mut ffmpeg::codec::decoder::Audio,
    resampler: &mut ffmpeg::software::resampling::Context,
    decoded: &mut ffmpeg::frame::Audio,
    samples: &mut Vec<i32>,
    output_layout: ffmpeg::ChannelLayout,
    output_rate: u32,
) -> Result<()> {
    while decoder.receive_frame(decoded).is_ok() {
        resample_frame(resampler, decoded, samples, output_layout, output_rate)?;
    }
    Ok(())
}

fn resample_frame(
    resampler: &mut ffmpeg::software::resampling::Context,
    decoded: &ffmpeg::frame::Audio,
    samples: &mut Vec<i32>,
    output_layout: ffmpeg::ChannelLayout,
    output_rate: u32,
) -> Result<()> {
    let input_layout = normalize_layout(decoded.channel_layout(), decoded.channels());
    let input_rate = decoded.rate();

    // Fast path: decoded frame already matches our target PCM format.
    if decoded.format() == ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed)
        && input_layout == output_layout
        && input_rate == output_rate
    {
        return append_packed_i16_frame(decoded, samples);
    }

    // Some real-world streams (especially containerized/transcoded assets) can
    // trigger repeated InputChanged/OutputChanged notifications while decoder
    // parameters settle. Rebuild and retry a few times before failing hard.
    for _attempt in 0..3 {
        let mut output = ffmpeg::frame::Audio::empty();
        match resampler.run(decoded, &mut output) {
            Ok(_) => return append_packed_i16_frame(&output, samples),
            Err(ffmpeg::Error::InputChanged | ffmpeg::Error::OutputChanged) => {
                // Some frames report rate=0 after parameter switch; fall back to
                // target output rate to keep resampler reconfiguration valid.
                let safe_input_rate = input_rate.max(output_rate);
                *resampler = create_resampler(
                    decoded.format(),
                    input_layout,
                    safe_input_rate,
                    output_layout,
                    output_rate,
                )?;
            }
            Err(err) => return Err(Error::FfmpegDecodeFailed(format!("resample failed: {err}"))),
        }
    }

    // Last resort: build a one-shot resampler from the current frame params.
    let safe_input_rate = input_rate.max(output_rate);
    let mut one_shot = create_resampler(
        decoded.format(),
        input_layout,
        safe_input_rate,
        output_layout,
        output_rate,
    )?;
    let mut output = ffmpeg::frame::Audio::empty();
    match one_shot.run(decoded, &mut output) {
        Ok(_) => append_packed_i16_frame(&output, samples),
        Err(ffmpeg::Error::InputChanged | ffmpeg::Error::OutputChanged)
            if decoded.format()
                == ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed)
                && input_layout == output_layout =>
        {
            append_packed_i16_frame(decoded, samples)
        }
        Err(err) => Err(Error::FfmpegDecodeFailed(format!(
            "resample failed after fallback: {err}"
        ))),
    }
}

fn create_resampler(
    src_format: ffmpeg::format::Sample,
    src_layout: ffmpeg::ChannelLayout,
    src_rate: u32,
    dst_layout: ffmpeg::ChannelLayout,
    dst_rate: u32,
) -> Result<ffmpeg::software::resampling::Context> {
    ffmpeg::software::resampling::Context::get(
        src_format,
        src_layout,
        src_rate,
        ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed),
        dst_layout,
        dst_rate,
    )
    .map_err(|err| Error::FfmpegDecodeFailed(format!("failed to create audio resampler: {err}")))
}

fn normalize_layout(layout: ffmpeg::ChannelLayout, channels: u16) -> ffmpeg::ChannelLayout {
    if layout.bits() == 0 {
        ffmpeg::ChannelLayout::default(i32::from(channels))
    } else {
        layout
    }
}

fn flush_resampler(
    resampler: &mut ffmpeg::software::resampling::Context,
    samples: &mut Vec<i32>,
) -> Result<()> {
    loop {
        let mut flushed = ffmpeg::frame::Audio::empty();
        match resampler.flush(&mut flushed) {
            Ok(_) => {
                if flushed.samples() == 0 {
                    break;
                }
                append_packed_i16_frame(&flushed, samples)?;
            }
            // 某些容器/轨道在 flush 阶段会返回参数切换信号；这里按“无更多可刷数据”处理。
            Err(
                ffmpeg::Error::OutputChanged | ffmpeg::Error::InputChanged | ffmpeg::Error::Eof,
            ) => {
                break;
            }
            Err(err) => {
                return Err(Error::FfmpegDecodeFailed(format!(
                    "resampler flush failed: {err}"
                )));
            }
        }
    }
    Ok(())
}

fn append_packed_i16_frame(frame: &ffmpeg::frame::Audio, samples: &mut Vec<i32>) -> Result<()> {
    let channels = usize::from(frame.channels());
    let sample_count = frame.samples();
    if channels == 0 || sample_count == 0 {
        return Ok(());
    }

    let expected_bytes = sample_count
        .checked_mul(channels)
        .and_then(|value| value.checked_mul(std::mem::size_of::<i16>()))
        .ok_or_else(|| Error::FfmpegDecodeFailed("decoded frame size overflow".to_string()))?;

    let data = frame.data(0);
    if data.len() < expected_bytes {
        return Err(Error::FfmpegDecodeFailed(format!(
            "decoded frame is truncated: expected {expected_bytes} bytes, got {}",
            data.len()
        )));
    }

    for chunk in data[..expected_bytes].chunks_exact(2) {
        let sample = i16::from_ne_bytes([chunk[0], chunk[1]]);
        samples.push(i32::from(sample));
    }
    Ok(())
}

#[allow(unsafe_code)]
fn has_demuxer(name: &str) -> bool {
    let Ok(c_name) = CString::new(name) else {
        return false;
    };
    // SAFETY: av_find_input_format 只读取传入的 null-terminated 字符串。
    unsafe { !ffmpeg::ffi::av_find_input_format(c_name.as_ptr()).is_null() }
}
