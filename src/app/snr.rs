use std::path::Path;
#[cfg(feature = "ffmpeg-decode")]
use std::sync::OnceLock;

#[cfg(feature = "ffmpeg-decode")]
use ffmpeg_next as ffmpeg;

pub const SNR_STATUS_OK: &str = "ok";
pub const SNR_STATUS_UNAVAILABLE: &str = "unavailable";
pub const SNR_STATUS_ERROR: &str = "error";

#[cfg(feature = "ffmpeg-decode")]
/// Internal constant.
const SNR_TARGET_SAMPLE_RATE: u32 = 48_000;
#[cfg(feature = "ffmpeg-decode")]
/// Internal constant.
const SNR_MIN_OVERLAP_SAMPLES: usize = 4_800;
#[cfg(feature = "ffmpeg-decode")]
/// Internal item.
static FFMPEG_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct SnrAnalysis {
    pub snr_db: Option<f64>,
    pub status: String,
    pub detail: Option<String>,
}

impl SnrAnalysis {
    #[must_use]
    pub fn ok(snr_db: f64) -> Self {
        Self {
            snr_db: Some(snr_db),
            status: SNR_STATUS_OK.to_string(),
            detail: None,
        }
    }

    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self {
            snr_db: None,
            status: SNR_STATUS_UNAVAILABLE.to_string(),
            detail: Some(detail.into()),
        }
    }

    pub fn error(detail: impl Into<String>) -> Self {
        Self {
            snr_db: None,
            status: SNR_STATUS_ERROR.to_string(),
            detail: Some(detail.into()),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn analyze_snr<P: AsRef<Path>>(input: P, output: P) -> SnrAnalysis {
    #[cfg(not(feature = "ffmpeg-decode"))]
    {
        let _ = input;
        let _ = output;
        return SnrAnalysis::unavailable("ffmpeg_decode_feature_disabled");
    }

    #[cfg(feature = "ffmpeg-decode")]
    let input_samples = match decode_media_to_i16_mono_via_avfilter(input.as_ref()) {
        Ok(value) => value,
        Err(error) => return SnrAnalysis::unavailable(format!("input_decode_failed:{error}")),
    };

    #[cfg(feature = "ffmpeg-decode")]
    let output_samples = match decode_media_to_i16_mono_via_avfilter(output.as_ref()) {
        Ok(value) => value,
        Err(error) => return SnrAnalysis::unavailable(format!("output_decode_failed:{error}")),
    };

    if input_samples.is_empty() || output_samples.is_empty() {
        return SnrAnalysis::unavailable("empty_audio");
    }

    let overlap = input_samples.len().min(output_samples.len());
    if overlap < SNR_MIN_OVERLAP_SAMPLES {
        return SnrAnalysis::unavailable("insufficient_overlap");
    }

    let mut signal_power = 0.0_f64;
    let mut noise_power = 0.0_f64;
    let mut sample_count = 0.0_f64;

    for (input_sample, output_sample) in input_samples[..overlap]
        .iter()
        .zip(output_samples[..overlap].iter())
    {
        let signal = normalize_sample(*input_sample);
        let output_value = normalize_sample(*output_sample);
        let noise = signal - output_value;
        signal_power += signal * signal;
        noise_power += noise * noise;
        sample_count += 1.0;
    }

    if sample_count <= 0.0 {
        return SnrAnalysis::unavailable("empty_audio");
    }

    signal_power /= sample_count;
    noise_power /= sample_count;

    if !signal_power.is_finite() || !noise_power.is_finite() {
        return SnrAnalysis::error("non_finite_power");
    }

    if noise_power <= f64::EPSILON {
        return SnrAnalysis::ok(120.0);
    }

    if signal_power <= f64::EPSILON {
        return SnrAnalysis::unavailable("near_silence_input");
    }

    let snr_db = 10.0 * (signal_power / noise_power).log10();
    if !snr_db.is_finite() {
        return SnrAnalysis::error("non_finite_snr");
    }

    SnrAnalysis::ok(snr_db.clamp(-60.0, 120.0))
}

/// Internal helper function.
fn normalize_sample(sample: i16) -> f64 {
    f64::from(sample) / f64::from(i16::MAX)
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn decode_media_to_i16_mono_via_avfilter(path: &Path) -> std::result::Result<Vec<i16>, String> {
    ensure_ffmpeg_initialized()?;
    ensure_required_filters()?;

    let mut input_ctx = ffmpeg::format::input(path).map_err(|err| err.to_string())?;
    let stream = input_ctx
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or_else(|| "no_audio_stream".to_string())?;
    let stream_index = stream.index();

    let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|err| format!("codec_context:{err}"))?;
    let mut decoder = context
        .decoder()
        .audio()
        .map_err(|err| format!("decoder_open:{err}"))?;

    if decoder.rate() == 0 || decoder.channels() == 0 {
        return Err("invalid_stream_metadata".to_string());
    }

    let mut graph = create_audio_normalize_graph(&decoder)?;
    let mut decoded_frame = ffmpeg::frame::Audio::empty();
    let mut normalized = Vec::<i16>::new();

    for (packet_stream, packet) in input_ctx.packets() {
        if packet_stream.index() != stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .map_err(|err| format!("decoder_send_packet:{err}"))?;
        receive_decoded_frames_into_graph(
            &mut decoder,
            &mut graph,
            &mut decoded_frame,
            &mut normalized,
        )?;
    }

    decoder
        .send_eof()
        .map_err(|err| format!("decoder_send_eof:{err}"))?;
    receive_decoded_frames_into_graph(
        &mut decoder,
        &mut graph,
        &mut decoded_frame,
        &mut normalized,
    )?;

    let mut in_ctx = graph
        .get("in")
        .ok_or_else(|| "filter_input_not_found".to_string())?;
    in_ctx
        .source()
        .flush()
        .map_err(|err| format!("filter_flush:{err}"))?;
    drain_filtered_samples(&mut graph, &mut normalized)?;

    Ok(normalized)
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn receive_decoded_frames_into_graph(
    decoder: &mut ffmpeg::codec::decoder::Audio,
    graph: &mut ffmpeg::filter::Graph,
    frame: &mut ffmpeg::frame::Audio,
    output: &mut Vec<i16>,
) -> std::result::Result<(), String> {
    while decoder.receive_frame(frame).is_ok() {
        let layout = normalize_layout(frame.channel_layout(), frame.channels());
        if frame.channel_layout().bits() == 0 {
            frame.set_channel_layout(layout);
        }
        if frame.rate() == 0 {
            frame.set_rate(decoder.rate());
        }
        let timestamp = frame.timestamp();
        frame.set_pts(timestamp);

        let mut in_ctx = graph
            .get("in")
            .ok_or_else(|| "filter_input_not_found".to_string())?;
        in_ctx
            .source()
            .add(frame)
            .map_err(|err| format!("filter_add_frame:{err}"))?;
        drain_filtered_samples(graph, output)?;
    }
    Ok(())
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn drain_filtered_samples(
    graph: &mut ffmpeg::filter::Graph,
    output: &mut Vec<i16>,
) -> std::result::Result<(), String> {
    let mut filtered = ffmpeg::frame::Audio::empty();
    loop {
        let mut out_ctx = graph
            .get("out")
            .ok_or_else(|| "filter_output_not_found".to_string())?;
        match out_ctx.sink().frame(&mut filtered) {
            Ok(()) => append_i16_frame(&filtered, output)?,
            Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::util::error::EAGAIN => break,
            Err(ffmpeg::Error::Eof) => break,
            Err(err) => return Err(format!("filter_drain:{err}")),
        }
    }
    Ok(())
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn append_i16_frame(
    frame: &ffmpeg::frame::Audio,
    output: &mut Vec<i16>,
) -> std::result::Result<(), String> {
    let channels = usize::from(frame.channels());
    let sample_count = frame.samples();
    if channels == 0 || sample_count == 0 {
        return Ok(());
    }

    match frame.format() {
        ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed) => {
            let required = sample_count
                .checked_mul(channels)
                .and_then(|v| v.checked_mul(std::mem::size_of::<i16>()))
                .ok_or_else(|| "frame_size_overflow".to_string())?;
            let data = frame.data(0);
            if data.len() < required {
                return Err("frame_truncated".to_string());
            }
            for chunk in data[..required].chunks_exact(2) {
                output.push(i16::from_ne_bytes([chunk[0], chunk[1]]));
            }
            Ok(())
        }
        ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Planar) => {
            for sample_index in 0..sample_count {
                for channel_index in 0..channels {
                    let plane = frame.plane::<i16>(channel_index);
                    if let Some(sample) = plane.get(sample_index) {
                        output.push(*sample);
                    }
                }
            }
            Ok(())
        }
        _ => Err("unexpected_sample_format".to_string()),
    }
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn create_audio_normalize_graph(
    decoder: &ffmpeg::codec::decoder::Audio,
) -> std::result::Result<ffmpeg::filter::Graph, String> {
    let mut graph = ffmpeg::filter::Graph::new();
    let layout = normalize_layout(decoder.channel_layout(), decoder.channels());

    let args = format!(
        "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
        decoder.time_base(),
        decoder.rate(),
        decoder.format().name(),
        layout.bits()
    );

    let abuffer =
        ffmpeg::filter::find("abuffer").ok_or_else(|| "missing_filter_abuffer".to_string())?;
    let sink = ffmpeg::filter::find("abuffersink")
        .ok_or_else(|| "missing_filter_abuffersink".to_string())?;

    graph
        .add(&abuffer, "in", &args)
        .map_err(|err| format!("graph_add_abuffer:{err}"))?;
    graph
        .add(&sink, "out", "")
        .map_err(|err| format!("graph_add_abuffersink:{err}"))?;

    let spec =
        format!("aformat=sample_fmts=s16:channel_layouts=mono,aresample={SNR_TARGET_SAMPLE_RATE}");
    graph
        .output("in", 0)
        .map_err(|err| format!("graph_output:{err}"))?
        .input("out", 0)
        .map_err(|err| format!("graph_input:{err}"))?
        .parse(&spec)
        .map_err(|err| format!("graph_parse:{err}"))?;

    graph
        .validate()
        .map_err(|err| format!("graph_validate:{err}"))?;
    Ok(graph)
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn ensure_required_filters() -> std::result::Result<(), String> {
    for filter in ["abuffer", "abuffersink", "aformat", "aresample"] {
        if ffmpeg::filter::find(filter).is_none() {
            return Err(format!("missing_filter_{filter}"));
        }
    }
    Ok(())
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn ensure_ffmpeg_initialized() -> std::result::Result<(), String> {
    match FFMPEG_INIT.get_or_init(|| ffmpeg::init().map_err(|err| err.to_string())) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.clone()),
    }
}

#[cfg(feature = "ffmpeg-decode")]
/// Internal helper function.
fn normalize_layout(layout: ffmpeg::ChannelLayout, channels: u16) -> ffmpeg::ChannelLayout {
    if layout.bits() == 0 {
        ffmpeg::ChannelLayout::default(i32::from(channels))
    } else {
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_sample_is_bounded() {
        let value = normalize_sample(i16::MAX);
        assert!(value > 0.99 && value <= 1.0);
    }

    #[test]
    fn snr_analysis_ok_helper_sets_status() {
        let value = SnrAnalysis::ok(12.34);
        assert_eq!(value.status, SNR_STATUS_OK);
        assert_eq!(value.snr_db, Some(12.34));
    }
}
