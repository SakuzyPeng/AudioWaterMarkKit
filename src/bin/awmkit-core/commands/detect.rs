use crate::error::{CliError, Result};
use crate::util::{audio_from_context, ensure_file, expand_inputs, CliLayout};
use crate::Context;
use awmkit::app::{build_audio_proof, i18n, AppError, EvidenceStore, KeyStore};
use awmkit::ChannelLayout;
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;
use indicatif::{ProgressBar, ProgressStyle};
use rusty_chromaprint::{match_fingerprints, Configuration};
use serde::Serialize;

const CLONE_LIKELY_MAX_SCORE: f64 = 7.0;
const CLONE_LIKELY_MIN_SECONDS: f32 = 6.0;
const DETECT_PROGRESS_TEMPLATE: &str = "{prefix} [{bar:40}] {pos}/{len}";

#[derive(Args)]
pub struct DetectArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,

    /// Channel layout (default: auto)
    #[arg(long, value_enum, default_value_t = CliLayout::Auto)]
    pub layout: CliLayout,

    /// Input files (supports glob)
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

#[derive(Serialize)]
struct DetectJson {
    file: String,
    status: String,
    verification: Option<String>,
    forensic_warning: Option<String>,
    tag: Option<String>,
    identity: Option<String>,
    version: Option<u8>,
    key_slot: Option<u8>,
    timestamp_minutes: Option<u32>,
    timestamp_utc: Option<u64>,
    pattern: Option<String>,
    detect_score: Option<f32>,
    bit_errors: Option<u32>,
    match_found: Option<bool>,
    error: Option<String>,
    clone_check: Option<String>,
    clone_score: Option<f64>,
    clone_match_seconds: Option<f32>,
    clone_matched_evidence_id: Option<i64>,
    clone_reason: Option<String>,
    decode_slot_hint: Option<u8>,
    decode_slot_used: Option<u8>,
    slot_status: Option<String>,
    slot_scan_count: Option<u32>,
}

pub fn run(ctx: &Context, args: &DetectArgs) -> Result<()> {
    let inputs = expand_inputs(&args.inputs)?;
    for input in &inputs {
        ensure_file(input)?;
    }

    let key_store = KeyStore::new()?;
    let audio = audio_from_context(ctx)?;
    let layout = args.layout.to_channel_layout();
    let evidence_store = match EvidenceStore::load() {
        Ok(store) => Some(store),
        Err(err) => {
            ctx.out.warn(format!("[WARN] evidence: {err}"));
            None
        }
    };
    if ctx.out.verbose() && !ctx.out.quiet() {
        let parallelism = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(1);
        ctx.out.info(format!(
            "[INFO] multichannel route steps use Rayon parallel execution (max workers: {parallelism})"
        ));
    }

    if args.json {
        let mut results = Vec::new();
        for input in inputs {
            results.push(detect_one_json(
                &audio,
                &key_store,
                &input,
                layout,
                evidence_store.as_ref(),
            ));
        }
        let output = serde_json::to_string_pretty(&results)?;
        println!("{output}");
        return Ok(());
    }

    let progress = if ctx.out.quiet() {
        None
    } else {
        let bar = ProgressBar::new(inputs.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(DETECT_PROGRESS_TEMPLATE)
                .map_err(|e| CliError::Message(e.to_string()))?
                .progress_chars("=>-"),
        );
        bar.set_prefix("detect");
        Some(bar)
    };

    let mut ok = 0usize;
    let mut miss = 0usize;
    let mut invalid = 0usize;

    for input in inputs {
        match detect_one(&audio, &key_store, &input, layout, evidence_store.as_ref()) {
            Ok(DetectOutcome::Found {
                tag,
                identity,
                clone_check,
                detect_score,
                decode_slot_hint,
                decode_slot_used,
                slot_status,
                slot_scan_count,
            }) => {
                ok += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    let score_text = detect_score
                        .map(|score| format!(", score: {score:.3}"))
                        .unwrap_or_default();
                    if let Some(ref bar) = progress {
                        bar.println(format!(
                            "[OK] {} (tag: {tag}, id: {identity}, clone: {}{}, slot: hint={} used={} status={} scan={})",
                            input.display(),
                            clone_check.summary(),
                            score_text,
                            decode_slot_hint,
                            decode_slot_used,
                            slot_status,
                            slot_scan_count
                        ));
                    } else {
                        ctx.out.info(format!(
                            "[OK] {} (tag: {tag}, id: {identity}, clone: {}{}, slot: hint={} used={} status={} scan={})",
                            input.display(),
                            clone_check.summary(),
                            score_text,
                            decode_slot_hint,
                            decode_slot_used,
                            slot_status,
                            slot_scan_count
                        ));
                    }
                }
            }
            Ok(DetectOutcome::NotFound) => {
                miss += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    if let Some(ref bar) = progress {
                        bar.println(format!("[MISS] {}", input.display()));
                    } else {
                        ctx.out.info(format!("[MISS] {}", input.display()));
                    }
                }
            }
            Ok(DetectOutcome::Invalid {
                error,
                unverified,
                detect_score,
                decode_slot_hint,
                decode_slot_used,
                slot_status,
                slot_scan_count,
            }) => {
                invalid += 1;
                let score_text = detect_score
                    .map(|score| format!(" (score: {score:.3})"))
                    .unwrap_or_default();
                let decoded_text = unverified.as_ref().map_or_else(
                    || " (tag=- id=- time=- slot=-)".to_string(),
                    |decoded| {
                        format!(
                            " (tag={} id={} time={} slot={})",
                            decoded.tag,
                            decoded.identity(),
                            decoded.timestamp_utc,
                            decoded.key_slot
                        )
                    },
                );
                let slot_text = format!(
                    " (slot: hint={} used={} status={} scan={})",
                    decode_slot_hint.map_or_else(|| "-".to_string(), |value| value.to_string()),
                    decode_slot_used.map_or_else(|| "-".to_string(), |value| value.to_string()),
                    slot_status,
                    slot_scan_count
                );
                if let Some(ref bar) = progress {
                    bar.println(format!(
                        "[INVALID] {}: {}{}{}{} [UNVERIFIED] {}",
                        input.display(),
                        error,
                        score_text,
                        decoded_text,
                        slot_text,
                        i18n::tr("cli-detect-forensic-warning")
                    ));
                } else {
                    crate::output::Output::error(format!(
                        "[INVALID] {}: {}{}{}{} [UNVERIFIED] {}",
                        input.display(),
                        error,
                        score_text,
                        decoded_text,
                        slot_text,
                        i18n::tr("cli-detect-forensic-warning")
                    ));
                }
            }
            Err(err) => {
                invalid += 1;
                if let Some(ref bar) = progress {
                    bar.println(format!("[ERR] {}: {err}", input.display()));
                } else {
                    crate::output::Output::error(format!("[ERR] {}: {err}", input.display()));
                }
            }
        }

        if let Some(ref bar) = progress {
            bar.inc(1);
        }
    }

    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    if !ctx.out.quiet() {
        let mut args = FluentArgs::new();
        args.set("ok", ok.to_string());
        args.set("miss", miss.to_string());
        args.set("invalid", invalid.to_string());
        ctx.out.info(i18n::tr_args("cli-detect-done", &args));
    }

    if invalid > 0 {
        Err(CliError::Message(i18n::tr("cli-detect-failed")))
    } else {
        Ok(())
    }
}

enum DetectOutcome {
    Found {
        tag: String,
        identity: String,
        clone_check: CloneCheck,
        detect_score: Option<f32>,
        decode_slot_hint: u8,
        decode_slot_used: u8,
        slot_status: String,
        slot_scan_count: u32,
    },
    NotFound,
    Invalid {
        error: String,
        unverified: Option<awmkit::MessageResult>,
        detect_score: Option<f32>,
        decode_slot_hint: Option<u8>,
        decode_slot_used: Option<u8>,
        slot_status: String,
        slot_scan_count: u32,
    },
}

#[derive(Clone)]
struct CloneCheck {
    check: String,
    score: Option<f64>,
    match_seconds: Option<f32>,
    matched_evidence_id: Option<i64>,
    reason: Option<String>,
}

impl CloneCheck {
    fn exact(matched_evidence_id: i64) -> Self {
        Self {
            check: "exact".to_string(),
            score: None,
            match_seconds: None,
            matched_evidence_id: Some(matched_evidence_id),
            reason: None,
        }
    }

    fn likely(matched_evidence_id: i64, score: f64, match_seconds: f32) -> Self {
        Self {
            check: "likely".to_string(),
            score: Some(score),
            match_seconds: Some(match_seconds),
            matched_evidence_id: Some(matched_evidence_id),
            reason: None,
        }
    }

    fn suspect(score: Option<f64>, match_seconds: Option<f32>, reason: &str) -> Self {
        Self {
            check: "suspect".to_string(),
            score,
            match_seconds,
            matched_evidence_id: None,
            reason: Some(reason.to_string()),
        }
    }

    fn unavailable(reason: String) -> Self {
        Self {
            check: "unavailable".to_string(),
            score: None,
            match_seconds: None,
            matched_evidence_id: None,
            reason: Some(reason),
        }
    }

    fn summary(&self) -> String {
        match self.check.as_str() {
            "exact" => "exact".to_string(),
            "likely" => {
                let score = self
                    .score
                    .map_or_else(|| "-".to_string(), |value| format!("{value:.2}"));
                let seconds = self
                    .match_seconds
                    .map_or_else(|| "-".to_string(), |value| format!("{value:.1}s"));
                format!("likely(score={score}, dur={seconds})")
            }
            "suspect" => {
                self.reason.as_ref().map_or_else(
                    || "suspect".to_string(),
                    |reason| format!("suspect({reason})"),
                )
            }
            "unavailable" => {
                self.reason.as_ref().map_or_else(
                    || "unavailable".to_string(),
                    |reason| format!("unavailable({reason})"),
                )
            }
            other => other.to_string(),
        }
    }
}

fn detect_one(
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    input: &std::path::Path,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
) -> Result<DetectOutcome> {
    match detect_best(audio, input, layout)? {
        None => Ok(DetectOutcome::NotFound),
        Some(result) => {
            let slot_resolution = resolve_decode_slot(&result.raw_message, key_store);
            match slot_resolution {
                SlotResolution::Decoded(decoded) => {
                    let clone_check = evaluate_clone_check(input, &decoded.message, evidence_store);
                    Ok(DetectOutcome::Found {
                        tag: decoded.message.tag.to_string(),
                        identity: decoded.message.identity().to_string(),
                        clone_check,
                        detect_score: result.detect_score,
                        decode_slot_hint: decoded.slot_hint,
                        decode_slot_used: decoded.slot_used,
                        slot_status: decoded.status,
                        slot_scan_count: decoded.scan_count,
                    })
                }
                SlotResolution::Invalid(invalid) => Ok(DetectOutcome::Invalid {
                    error: invalid.error,
                    unverified: Message::decode_unverified(&result.raw_message).ok(),
                    detect_score: result.detect_score,
                    decode_slot_hint: Some(invalid.slot_hint),
                    decode_slot_used: invalid.slot_used,
                    slot_status: invalid.status,
                    slot_scan_count: invalid.scan_count,
                }),
            }
        }
    }
}

fn detect_one_json(
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    input: &std::path::Path,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
) -> DetectJson {
    match detect_best(audio, input, layout) {
        Ok(None) => DetectJson {
            file: input.display().to_string(),
            status: "not_found".to_string(),
            verification: None,
            forensic_warning: None,
            tag: None,
            identity: None,
            version: None,
            key_slot: None,
            timestamp_minutes: None,
            timestamp_utc: None,
            pattern: None,
            detect_score: None,
            bit_errors: None,
            match_found: None,
            error: None,
            clone_check: None,
            clone_score: None,
            clone_match_seconds: None,
            clone_matched_evidence_id: None,
            clone_reason: None,
            decode_slot_hint: None,
            decode_slot_used: None,
            slot_status: None,
            slot_scan_count: None,
        },
        Ok(Some(result)) => match resolve_decode_slot(&result.raw_message, key_store) {
            SlotResolution::Decoded(decoded) => {
                let clone_check = evaluate_clone_check(input, &decoded.message, evidence_store);
                DetectJson {
                    file: input.display().to_string(),
                    status: "ok".to_string(),
                    verification: Some("verified".to_string()),
                    forensic_warning: None,
                    tag: Some(decoded.message.tag.to_string()),
                    identity: Some(decoded.message.identity().to_string()),
                    version: Some(decoded.message.version),
                    key_slot: Some(decoded.message.key_slot),
                    timestamp_minutes: Some(decoded.message.timestamp_minutes),
                    timestamp_utc: Some(decoded.message.timestamp_utc),
                    pattern: Some(result.pattern),
                    detect_score: result.detect_score,
                    bit_errors: Some(result.bit_errors),
                    match_found: Some(result.match_found),
                    error: None,
                    clone_check: Some(clone_check.check),
                    clone_score: clone_check.score,
                    clone_match_seconds: clone_check.match_seconds,
                    clone_matched_evidence_id: clone_check.matched_evidence_id,
                    clone_reason: clone_check.reason,
                    decode_slot_hint: Some(decoded.slot_hint),
                    decode_slot_used: Some(decoded.slot_used),
                    slot_status: Some(decoded.status),
                    slot_scan_count: Some(decoded.scan_count),
                }
            }
            SlotResolution::Invalid(invalid) => {
                let unverified = Message::decode_unverified(&result.raw_message).ok();
                DetectJson {
                    file: input.display().to_string(),
                    status: "invalid_hmac".to_string(),
                    verification: Some("unverified".to_string()),
                    forensic_warning: Some(i18n::tr("cli-detect-forensic-warning")),
                    tag: unverified.as_ref().map(|message| message.tag.to_string()),
                    identity: unverified
                        .as_ref()
                        .map(|message| message.identity().to_string()),
                    version: unverified.as_ref().map(|message| message.version),
                    key_slot: unverified.as_ref().map(|message| message.key_slot),
                    timestamp_minutes: unverified.as_ref().map(|message| message.timestamp_minutes),
                    timestamp_utc: unverified.as_ref().map(|message| message.timestamp_utc),
                    pattern: Some(result.pattern),
                    detect_score: result.detect_score,
                    bit_errors: Some(result.bit_errors),
                    match_found: Some(result.match_found),
                    error: Some(invalid.error),
                    clone_check: None,
                    clone_score: None,
                    clone_match_seconds: None,
                    clone_matched_evidence_id: None,
                    clone_reason: None,
                    decode_slot_hint: Some(invalid.slot_hint),
                    decode_slot_used: invalid.slot_used,
                    slot_status: Some(invalid.status),
                    slot_scan_count: Some(invalid.scan_count),
                }
            }
        },
        Err(err) => DetectJson {
            file: input.display().to_string(),
            status: "error".to_string(),
            verification: None,
            forensic_warning: None,
            tag: None,
            identity: None,
            version: None,
            key_slot: None,
            timestamp_minutes: None,
            timestamp_utc: None,
            pattern: None,
            detect_score: None,
            bit_errors: None,
            match_found: None,
            error: Some(err.to_string()),
            clone_check: None,
            clone_score: None,
            clone_match_seconds: None,
            clone_matched_evidence_id: None,
            clone_reason: None,
            decode_slot_hint: None,
            decode_slot_used: None,
            slot_status: None,
            slot_scan_count: None,
        },
    }
}

fn detect_best(
    audio: &awmkit::Audio,
    input: &std::path::Path,
    layout: Option<ChannelLayout>,
) -> Result<Option<awmkit::DetectResult>> {
    let result = audio.detect_multichannel(input, layout)?;
    Ok(result.best)
}

struct DecodedSlotMessage {
    message: awmkit::MessageResult,
    slot_hint: u8,
    slot_used: u8,
    status: String,
    scan_count: u32,
}

struct InvalidSlotDecode {
    slot_hint: u8,
    slot_used: Option<u8>,
    status: String,
    scan_count: u32,
    error: String,
}

enum SlotResolution {
    Decoded(DecodedSlotMessage),
    Invalid(InvalidSlotDecode),
}

fn resolve_decode_slot(message: &[u8], key_store: &KeyStore) -> SlotResolution {
    let slot_hint = match Message::peek_version_and_slot(message) {
        Ok((_, slot)) => slot,
        Err(err) => {
            return SlotResolution::Invalid(InvalidSlotDecode {
                slot_hint: 0,
                slot_used: None,
                status: "mismatch".to_string(),
                scan_count: 0,
                error: err.to_string(),
            });
        }
    };

    let mut candidate_slots = vec![slot_hint];
    for slot in key_store.list_configured_slots() {
        if slot != slot_hint {
            candidate_slots.push(slot);
        }
    }

    let mut decode_successes: Vec<(u8, awmkit::MessageResult)> = Vec::new();
    let mut scan_count: u32 = 0;
    let mut hint_key_missing = false;

    for slot in candidate_slots {
        match key_store.load_slot(slot) {
            Ok(key) => {
                scan_count = scan_count.saturating_add(1);
                if let Ok(decoded) = Message::decode(message, &key) {
                    decode_successes.push((slot, decoded));
                }
            }
            Err(AppError::KeyNotFound) => {
                if slot == slot_hint {
                    hint_key_missing = true;
                }
            }
            Err(_) => {}
        }
    }

    match decode_successes.len() {
        1 => {
            let (slot_used, decoded) = decode_successes.remove(0);
            let status = if slot_used == slot_hint {
                "matched".to_string()
            } else {
                "recovered".to_string()
            };
            SlotResolution::Decoded(DecodedSlotMessage {
                message: decoded,
                slot_hint,
                slot_used,
                status,
                scan_count,
            })
        }
        0 => {
            let (status, error) = if hint_key_missing {
                (
                    "missing_key".to_string(),
                    format!("key not found for slot {slot_hint}"),
                )
            } else {
                (
                    "mismatch".to_string(),
                    format!("decode failed after scanning {scan_count} slot(s)"),
                )
            };
            SlotResolution::Invalid(InvalidSlotDecode {
                slot_hint,
                slot_used: None,
                status,
                scan_count,
                error,
            })
        }
        _ => SlotResolution::Invalid(InvalidSlotDecode {
            slot_hint,
            slot_used: None,
            status: "ambiguous".to_string(),
            scan_count,
            error: "decoded by multiple slots".to_string(),
        }),
    }
}

fn evaluate_clone_check(
    input: &std::path::Path,
    decoded: &awmkit::MessageResult,
    evidence_store: Option<&EvidenceStore>,
) -> CloneCheck {
    let Some(evidence_store) = evidence_store else {
        return CloneCheck::unavailable("evidence_store_unavailable".to_string());
    };

    let proof = match build_audio_proof(input) {
        Ok(proof) => proof,
        Err(err) => return CloneCheck::unavailable(format!("proof_error: {err}")),
    };

    let candidates = match evidence_store.list_candidates(decoded.identity(), decoded.key_slot) {
        Ok(candidates) => candidates,
        Err(err) => return CloneCheck::unavailable(format!("query_error: {err}")),
    };

    if candidates.is_empty() {
        return CloneCheck::suspect(None, None, "no_evidence");
    }

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.pcm_sha256 == proof.pcm_sha256)
    {
        return CloneCheck::exact(candidate.id);
    }

    let config = Configuration::default();
    let mut best_match: Option<(i64, f64, f32)> = None;

    for candidate in &candidates {
        if candidate.fp_config_id != config.id() {
            continue;
        }

        let segments = match match_fingerprints(&proof.chromaprint, &candidate.chromaprint, &config)
        {
            Ok(segments) => segments,
            Err(err) => return CloneCheck::unavailable(format!("match_error: {err}")),
        };

        for segment in segments {
            let duration = segment.duration(&config);
            let score = segment.score;
            match best_match {
                None => best_match = Some((candidate.id, score, duration)),
                Some((_, best_score, best_duration))
                    if duration > best_duration
                        || ((duration - best_duration).abs() < f32::EPSILON
                            && score < best_score) =>
                {
                    best_match = Some((candidate.id, score, duration));
                }
                _ => {}
            }
        }
    }

    if let Some((candidate_id, score, duration)) = best_match {
        if is_likely(score, duration) {
            CloneCheck::likely(candidate_id, score, duration)
        } else {
            CloneCheck::suspect(Some(score), Some(duration), "threshold_not_met")
        }
    } else {
        CloneCheck::suspect(None, None, "no_similar_segment")
    }
}

fn is_likely(score: f64, match_seconds: f32) -> bool {
    score <= CLONE_LIKELY_MAX_SCORE && match_seconds >= CLONE_LIKELY_MIN_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn likely_threshold_boundary() {
        assert!(is_likely(7.0, 6.0));
        assert!(is_likely(1.5, 8.0));
        assert!(!is_likely(7.1, 6.0));
        assert!(!is_likely(7.0, 5.9));
    }
}
