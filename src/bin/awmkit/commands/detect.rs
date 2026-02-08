use crate::error::{CliError, Result};
use crate::util::{audio_from_context, ensure_file, expand_inputs};
use crate::Context;
use awmkit::app::{build_audio_proof, i18n, EvidenceStore, KeyStore};
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;
use indicatif::{ProgressBar, ProgressStyle};
use rusty_chromaprint::{match_fingerprints, Configuration};
use serde::Serialize;

const CLONE_LIKELY_MAX_SCORE: f64 = 7.0;
const CLONE_LIKELY_MIN_SECONDS: f32 = 6.0;

#[derive(Args)]
pub struct DetectArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,

    /// Input files (supports glob)
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

#[derive(Serialize)]
struct DetectJson {
    file: String,
    status: String,
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
}

pub fn run(ctx: &Context, args: &DetectArgs) -> Result<()> {
    let inputs = expand_inputs(&args.inputs)?;
    for input in &inputs {
        ensure_file(input)?;
    }

    let store = KeyStore::new()?;
    let key = store.load()?;
    let audio = audio_from_context(ctx)?;
    let evidence_store = match EvidenceStore::load() {
        Ok(store) => Some(store),
        Err(err) => {
            ctx.out.warn(format!("[WARN] evidence: {err}"));
            None
        }
    };

    if args.json {
        let mut results = Vec::new();
        for input in inputs {
            results.push(detect_one_json(
                &audio,
                &key,
                &input,
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
            ProgressStyle::with_template("{prefix} [{bar:40}] {pos}/{len}")
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
        match detect_one(&audio, &key, &input, evidence_store.as_ref()) {
            Ok(DetectOutcome::Found {
                tag,
                identity,
                clone_check,
                detect_score,
            }) => {
                ok += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    let score_text = detect_score
                        .map(|score| format!(", score: {score:.3}"))
                        .unwrap_or_default();
                    if let Some(ref bar) = progress {
                        bar.println(format!(
                            "[OK] {} (tag: {tag}, id: {identity}, clone: {}{})",
                            input.display(),
                            clone_check.summary(),
                            score_text
                        ));
                    } else {
                        ctx.out.info(format!(
                            "[OK] {} (tag: {tag}, id: {identity}, clone: {}{})",
                            input.display(),
                            clone_check.summary(),
                            score_text
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
                detect_score,
            }) => {
                invalid += 1;
                let score_text = detect_score
                    .map(|score| format!(" (score: {score:.3})"))
                    .unwrap_or_default();
                if let Some(ref bar) = progress {
                    bar.println(format!(
                        "[INVALID] {}: {}{}",
                        input.display(),
                        error,
                        score_text
                    ));
                } else {
                    ctx.out.error(format!(
                        "[INVALID] {}: {}{}",
                        input.display(),
                        error,
                        score_text
                    ));
                }
            }
            Err(err) => {
                invalid += 1;
                if let Some(ref bar) = progress {
                    bar.println(format!("[ERR] {}: {err}", input.display()));
                } else {
                    ctx.out.error(format!("[ERR] {}: {err}", input.display()));
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
    },
    NotFound,
    Invalid {
        error: String,
        detect_score: Option<f32>,
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
                if let Some(reason) = self.reason.as_ref() {
                    format!("suspect({reason})")
                } else {
                    "suspect".to_string()
                }
            }
            "unavailable" => {
                if let Some(reason) = self.reason.as_ref() {
                    format!("unavailable({reason})")
                } else {
                    "unavailable".to_string()
                }
            }
            other => other.to_string(),
        }
    }
}

fn detect_one(
    audio: &awmkit::Audio,
    key: &[u8],
    input: &std::path::Path,
    evidence_store: Option<&EvidenceStore>,
) -> Result<DetectOutcome> {
    match audio.detect(input)? {
        None => Ok(DetectOutcome::NotFound),
        Some(result) => match Message::decode(&result.raw_message, key) {
            Ok(decoded) => {
                let clone_check = evaluate_clone_check(input, &decoded, evidence_store);
                Ok(DetectOutcome::Found {
                    tag: decoded.tag.to_string(),
                    identity: decoded.identity().to_string(),
                    clone_check,
                    detect_score: result.detect_score,
                })
            }
            Err(err) => Ok(DetectOutcome::Invalid {
                error: err.to_string(),
                detect_score: result.detect_score,
            }),
        },
    }
}

fn detect_one_json(
    audio: &awmkit::Audio,
    key: &[u8],
    input: &std::path::Path,
    evidence_store: Option<&EvidenceStore>,
) -> DetectJson {
    match audio.detect(input) {
        Ok(None) => DetectJson {
            file: input.display().to_string(),
            status: "not_found".to_string(),
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
        },
        Ok(Some(result)) => match Message::decode(&result.raw_message, key) {
            Ok(decoded) => {
                let clone_check = evaluate_clone_check(input, &decoded, evidence_store);
                DetectJson {
                    file: input.display().to_string(),
                    status: "ok".to_string(),
                    tag: Some(decoded.tag.to_string()),
                    identity: Some(decoded.identity().to_string()),
                    version: Some(decoded.version),
                    key_slot: Some(decoded.key_slot),
                    timestamp_minutes: Some(decoded.timestamp_minutes),
                    timestamp_utc: Some(decoded.timestamp_utc),
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
                }
            }
            Err(err) => DetectJson {
                file: input.display().to_string(),
                status: "invalid_hmac".to_string(),
                tag: None,
                identity: None,
                version: None,
                key_slot: None,
                timestamp_minutes: None,
                timestamp_utc: None,
                pattern: Some(result.pattern),
                detect_score: result.detect_score,
                bit_errors: Some(result.bit_errors),
                match_found: Some(result.match_found),
                error: Some(err.to_string()),
                clone_check: None,
                clone_score: None,
                clone_match_seconds: None,
                clone_matched_evidence_id: None,
                clone_reason: None,
            },
        },
        Err(err) => DetectJson {
            file: input.display().to_string(),
            status: "error".to_string(),
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
        },
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
