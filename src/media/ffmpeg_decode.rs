//! FFmpeg 动态库解码后端

use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

use ffmpeg_next as ffmpeg;

use crate::audio::{AudioMediaCapabilities, DecodedPcm};
use crate::error::{Error, Result};

static FFMPEG_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
const WAV_PIPE_UNKNOWN_SIZE: u32 = u32::MAX;

struct DecodeContext {
    input_ctx: ffmpeg::format::context::Input,
    decoder: ffmpeg::codec::decoder::Audio,
    stream_index: usize,
    sample_rate: u32,
    channels: u16,
    output_layout: ffmpeg::ChannelLayout,
    output_rate: u32,
    resampler: ffmpeg::software::resampling::Context,
}

pub fn decode_media_to_pcm_i32(input: &Path) -> Result<DecodedPcm> {
    let mut context = open_decode_context(input)?;
    let mut samples = Vec::<i32>::new();
    let copied = decode_with_sink(&mut context, |bytes| {
        append_packed_i16_bytes(bytes, &mut samples);
        Ok(())
    })?;

    if copied == 0 {
        return Err(Error::FfmpegDecodeFailed(
            "no decodable audio samples found".to_string(),
        ));
    }

    Ok(DecodedPcm {
        sample_rate: context.sample_rate,
        channels: context.channels,
        bits_per_sample: 16,
        samples,
    })
}

pub fn decode_media_to_wav_pipe(input: &Path, writer: &mut dyn Write) -> Result<()> {
    let mut context = open_decode_context(input)?;
    write_wav_pipe_header(writer, context.sample_rate, context.channels)?;
    let copied = decode_with_sink(&mut context, |bytes| {
        writer.write_all(bytes)?;
        Ok(())
    })?;
    writer.flush()?;

    if copied == 0 {
        return Err(Error::FfmpegDecodeFailed(
            "no decodable audio samples found".to_string(),
        ));
    }

    Ok(())
}

pub fn media_capabilities() -> AudioMediaCapabilities {
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

fn open_decode_context(input: &Path) -> Result<DecodeContext> {
    ensure_ffmpeg_initialized()?;

    let input_ctx =
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

    let codec_context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|err| Error::FfmpegDecodeFailed(format!("failed to load codec context: {err}")))?;
    let decoder = codec_context
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
    let resampler = create_resampler(
        decoder.format(),
        output_layout,
        sample_rate,
        output_layout,
        output_rate,
    )?;

    Ok(DecodeContext {
        input_ctx,
        decoder,
        stream_index,
        sample_rate,
        channels,
        output_layout,
        output_rate,
        resampler,
    })
}

fn decode_with_sink<F>(context: &mut DecodeContext, mut sink: F) -> Result<usize>
where
    F: FnMut(&[u8]) -> Result<()>,
{
    let mut total_bytes = 0usize;
    let mut decoded_frame = ffmpeg::frame::Audio::empty();

    {
        let input_ctx = &mut context.input_ctx;
        let decoder = &mut context.decoder;
        let resampler = &mut context.resampler;
        let output_layout = context.output_layout;
        let output_rate = context.output_rate;
        let stream_index = context.stream_index;

        for (packet_stream, packet) in input_ctx.packets() {
            if packet_stream.index() != stream_index {
                continue;
            }

            decoder.send_packet(&packet).map_err(|err| {
                Error::FfmpegDecodeFailed(format!("decoder send packet failed: {err}"))
            })?;
            receive_decoded_frames(
                decoder,
                resampler,
                &mut decoded_frame,
                output_layout,
                output_rate,
                &mut total_bytes,
                &mut sink,
            )?;
        }

        decoder
            .send_eof()
            .map_err(|err| Error::FfmpegDecodeFailed(format!("decoder send eof failed: {err}")))?;
        receive_decoded_frames(
            decoder,
            resampler,
            &mut decoded_frame,
            output_layout,
            output_rate,
            &mut total_bytes,
            &mut sink,
        )?;
        flush_resampler(resampler, &mut total_bytes, &mut sink)?;
    }

    Ok(total_bytes)
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

fn receive_decoded_frames<F>(
    decoder: &mut ffmpeg::codec::decoder::Audio,
    resampler: &mut ffmpeg::software::resampling::Context,
    frame: &mut ffmpeg::frame::Audio,
    output_layout: ffmpeg::ChannelLayout,
    output_rate: u32,
    total_bytes: &mut usize,
    sink: &mut F,
) -> Result<()>
where
    F: FnMut(&[u8]) -> Result<()>,
{
    while decoder.receive_frame(frame).is_ok() {
        resample_frame(
            resampler,
            frame,
            output_layout,
            output_rate,
            total_bytes,
            sink,
        )?;
    }
    Ok(())
}

fn resample_frame<F>(
    resampler: &mut ffmpeg::software::resampling::Context,
    decoded: &ffmpeg::frame::Audio,
    output_layout: ffmpeg::ChannelLayout,
    output_rate: u32,
    total_bytes: &mut usize,
    sink: &mut F,
) -> Result<()>
where
    F: FnMut(&[u8]) -> Result<()>,
{
    let input_layout = normalize_layout(decoded.channel_layout(), decoded.channels());
    let input_rate = decoded.rate();

    // Fast path: decoded frame already matches our target PCM format.
    if decoded.format() == ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed)
        && input_layout == output_layout
        && input_rate == output_rate
    {
        return sink_frame_bytes(decoded, total_bytes, sink);
    }

    // Some real-world streams (especially containerized/transcoded assets) can
    // trigger repeated InputChanged/OutputChanged notifications while decoder
    // parameters settle. Rebuild and retry a few times before failing hard.
    for _attempt in 0..3 {
        let mut output = ffmpeg::frame::Audio::empty();
        match resampler.run(decoded, &mut output) {
            Ok(_) => return sink_frame_bytes(&output, total_bytes, sink),
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
        Ok(_) => sink_frame_bytes(&output, total_bytes, sink),
        Err(ffmpeg::Error::InputChanged | ffmpeg::Error::OutputChanged)
            if decoded.format()
                == ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed)
                && input_layout == output_layout =>
        {
            sink_frame_bytes(decoded, total_bytes, sink)
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

fn flush_resampler<F>(
    resampler: &mut ffmpeg::software::resampling::Context,
    total_bytes: &mut usize,
    sink: &mut F,
) -> Result<()>
where
    F: FnMut(&[u8]) -> Result<()>,
{
    loop {
        let mut flushed = ffmpeg::frame::Audio::empty();
        match resampler.flush(&mut flushed) {
            Ok(_) => {
                if flushed.samples() == 0 {
                    break;
                }
                sink_frame_bytes(&flushed, total_bytes, sink)?;
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

fn sink_frame_bytes<F>(
    frame: &ffmpeg::frame::Audio,
    total_bytes: &mut usize,
    sink: &mut F,
) -> Result<()>
where
    F: FnMut(&[u8]) -> Result<()>,
{
    let bytes = packed_i16_frame_bytes(frame)?;
    if bytes.is_empty() {
        return Ok(());
    }
    *total_bytes = total_bytes.saturating_add(bytes.len());
    sink(bytes)
}

fn packed_i16_frame_bytes(frame: &ffmpeg::frame::Audio) -> Result<&[u8]> {
    if frame.format() != ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed) {
        return Err(Error::FfmpegDecodeFailed(format!(
            "unexpected sample format {:?}, expected packed i16",
            frame.format()
        )));
    }

    let channels = usize::from(frame.channels());
    let sample_count = frame.samples();
    if channels == 0 || sample_count == 0 {
        return Ok(&[]);
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

    Ok(&data[..expected_bytes])
}

fn append_packed_i16_bytes(bytes: &[u8], samples: &mut Vec<i32>) {
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_ne_bytes([chunk[0], chunk[1]]);
        samples.push(i32::from(sample));
    }
}

fn write_wav_pipe_header(writer: &mut dyn Write, sample_rate: u32, channels: u16) -> Result<()> {
    if channels == 0 {
        return Err(Error::FfmpegDecodeFailed(
            "decoded audio metadata is invalid".to_string(),
        ));
    }
    let block_align = channels
        .checked_mul(2)
        .ok_or_else(|| Error::FfmpegDecodeFailed("wav header block_align overflow".to_string()))?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(block_align))
        .ok_or_else(|| Error::FfmpegDecodeFailed("wav header byte_rate overflow".to_string()))?;

    writer.write_all(b"RIFF")?;
    writer.write_all(&WAV_PIPE_UNKNOWN_SIZE.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&WAV_PIPE_UNKNOWN_SIZE.to_le_bytes())?;
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

#[cfg(test)]
mod tests {
    use super::write_wav_pipe_header;

    #[test]
    fn test_write_wav_pipe_header_layout() {
        let mut out = Vec::new();
        let wrote = write_wav_pipe_header(&mut out, 48_000, 2);
        assert!(wrote.is_ok());
        assert_eq!(out.len(), 44);
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WAVE");
        assert_eq!(&out[12..16], b"fmt ");
        assert_eq!(&out[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes([out[4], out[5], out[6], out[7]]),
            u32::MAX
        );
        assert_eq!(
            u32::from_le_bytes([out[40], out[41], out[42], out[43]]),
            u32::MAX
        );
    }
}
