use crate::media::decode_media_to_pcm_i32;
use std::path::Path;

pub const SNR_STATUS_OK: &str = "ok";
pub const SNR_STATUS_UNAVAILABLE: &str = "unavailable";
pub const SNR_STATUS_ERROR: &str = "error";

#[derive(Debug, Clone)]
pub struct SnrAnalysis {
    pub snr_db: Option<f64>,
    pub status: String,
    pub detail: Option<String>,
}

impl SnrAnalysis {
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

pub fn analyze_snr<P: AsRef<Path>>(input: P, output: P) -> SnrAnalysis {
    let input_audio = match decode_media_to_pcm_i32(input.as_ref()) {
        Ok(value) => value,
        Err(error) => return SnrAnalysis::unavailable(format!("input_decode_failed:{error}")),
    };
    let output_audio = match decode_media_to_pcm_i32(output.as_ref()) {
        Ok(value) => value,
        Err(error) => return SnrAnalysis::unavailable(format!("output_decode_failed:{error}")),
    };

    if input_audio.sample_rate != output_audio.sample_rate {
        return SnrAnalysis::unavailable("mismatch_sample_rate");
    }
    if input_audio.channels != output_audio.channels {
        return SnrAnalysis::unavailable("mismatch_channels");
    }
    if input_audio.samples.len() != output_audio.samples.len() {
        return SnrAnalysis::unavailable("mismatch_sample_count");
    }

    let input_samples = &input_audio.samples;
    let output_samples = &output_audio.samples;
    if input_samples.is_empty() || output_samples.is_empty() {
        return SnrAnalysis::unavailable("empty_audio");
    }
    if input_samples.len() != output_samples.len() {
        return SnrAnalysis::unavailable("mismatch_sample_count");
    }

    let mut signal_power = 0.0_f64;
    let mut noise_power = 0.0_f64;
    let mut count = 0_u64;

    for (input_sample, output_sample) in input_samples.iter().zip(output_samples.iter()) {
        let signal = normalize_sample(*input_sample, input_audio.bits_per_sample);
        let output_value = normalize_sample(*output_sample, output_audio.bits_per_sample);
        let noise = signal - output_value;
        signal_power += signal * signal;
        noise_power += noise * noise;
        count = count.saturating_add(1);
    }

    if count == 0 {
        return SnrAnalysis::unavailable("empty_audio");
    }

    let count_f64 = count as f64;
    signal_power /= count_f64;
    noise_power /= count_f64;

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

fn normalize_sample(sample: i32, bits_per_sample: u16) -> f64 {
    let clamped_bits = bits_per_sample.clamp(16, 32);
    let shift = i64::from(clamped_bits.saturating_sub(1));
    let denominator = (1_i64 << shift) as f64;
    f64::from(sample) / denominator
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn normalize_sample_is_bounded() {
        let value = normalize_sample(i32::MAX, 32);
        assert!(value > 0.99 && value <= 1.0);
    }

    #[test]
    fn snr_analysis_ok_helper_sets_status() {
        let value = SnrAnalysis::ok(12.34);
        assert_eq!(value.status, SNR_STATUS_OK);
        assert_eq!(value.snr_db, Some(12.34));
    }
}
