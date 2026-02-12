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

    let src_layout = if decoder.channel_layout().bits() == 0 {
        ffmpeg::ChannelLayout::default(i32::from(channels))
    } else {
        decoder.channel_layout()
    };

    let mut resampler = ffmpeg::software::resampling::Context::get(
        decoder.format(),
        src_layout,
        sample_rate,
        ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed),
        src_layout,
        sample_rate,
    )
    .map_err(|err| Error::FfmpegDecodeFailed(format!("failed to create audio resampler: {err}")))?;

    let mut decoded = ffmpeg::frame::Audio::empty();
    let mut samples = Vec::<i32>::new();

    for (packet_stream, packet) in input_ctx.packets() {
        if packet_stream.index() != stream_index {
            continue;
        }

        decoder.send_packet(&packet).map_err(|err| {
            Error::FfmpegDecodeFailed(format!("decoder send packet failed: {err}"))
        })?;
        receive_decoded_frames(&mut decoder, &mut resampler, &mut decoded, &mut samples)?;
    }

    decoder
        .send_eof()
        .map_err(|err| Error::FfmpegDecodeFailed(format!("decoder send eof failed: {err}")))?;
    receive_decoded_frames(&mut decoder, &mut resampler, &mut decoded, &mut samples)?;
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
) -> Result<()> {
    while decoder.receive_frame(decoded).is_ok() {
        let mut output = ffmpeg::frame::Audio::empty();
        resampler
            .run(decoded, &mut output)
            .map_err(|err| Error::FfmpegDecodeFailed(format!("resample failed: {err}")))?;
        append_packed_i16_frame(&output, samples)?;
    }
    Ok(())
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
