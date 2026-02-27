use crate::error::{CliError, Result};
use crate::util::{audio_from_context, ensure_file, expand_inputs, CliLayout};
use crate::Context;
use awmkit::app::{build_proof, i18n, EvidenceStore, Failure, KeyStore};
use awmkit::ChannelLayout;
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;
use indicatif::{ProgressBar, ProgressStyle};
use rusty_chromaprint::{match_fingerprints, Configuration};
use serde::Serialize;

/// Internal constant.
const CLONE_LIKELY_MAX_SCORE: f64 = 7.0;
/// Internal constant.
const CLONE_LIKELY_MIN_SECONDS: f32 = 6.0;
/// Internal constant.
const DETECT_PROGRESS_TEMPLATE: &str = "{prefix} [{bar:40}] {pos}/{len}";

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// JSON output.
    #[arg(long)]
    pub json: bool,

    /// Channel layout (default: auto).
    #[arg(long, value_enum, default_value_t = CliLayout::Auto)]
    pub layout: CliLayout,

    /// Input files (supports glob).
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

#[derive(Serialize)]
/// Internal struct.
struct DetectJson {
    /// Internal field.
    file: String,
    /// Internal field.
    status: String,
    /// Internal field.
    verification: Option<String>,
    /// Internal field.
    forensic_warning: Option<String>,
    /// Internal field.
    tag: Option<String>,
    /// Internal field.
    identity: Option<String>,
    /// Internal field.
    version: Option<u8>,
    /// Internal field.
    key_slot: Option<u8>,
    /// Internal field.
    timestamp_minutes: Option<u32>,
    /// Internal field.
    timestamp_utc: Option<u64>,
    /// Internal field.
    pattern: Option<String>,
    /// Internal field.
    detect_score: Option<f32>,
    /// Internal field.
    bit_errors: Option<u32>,
    /// Internal field.
    match_found: Option<bool>,
    /// Internal field.
    error: Option<String>,
    /// Internal field.
    clone_check: Option<String>,
    /// Internal field.
    clone_score: Option<f64>,
    /// Internal field.
    clone_match_seconds: Option<f32>,
    /// Internal field.
    clone_matched_evidence_id: Option<i64>,
    /// Internal field.
    clone_reason: Option<String>,
    /// Internal field.
    decode_slot_hint: Option<u8>,
    /// Internal field.
    decode_slot_used: Option<u8>,
    /// Internal field.
    slot_status: Option<String>,
    /// Internal field.
    slot_scan_count: Option<u32>,
    /// Internal field.
    detect_route: String,
    /// Internal field.
    fallback_triggered: bool,
    /// Internal field.
    fallback_reason: Option<String>,
}

/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
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
            let mut args_i18n = FluentArgs::new();
            args_i18n.set("error", err.to_string());
            ctx.out.warn_diag(i18n::tr_args(
                "cli-detect-evidence-store-unavailable-detail",
                &args_i18n,
            ));
            None
        }
    };
    log_parallelism(ctx);

    if args.json {
        run_json_mode(&inputs, &audio, &key_store, layout, evidence_store.as_ref())?;
        return Ok(());
    }

    let progress = build_progress(ctx, inputs.len())?;
    let stats = run_text_mode(
        ctx,
        &inputs,
        &audio,
        &key_store,
        layout,
        evidence_store.as_ref(),
        progress.as_ref(),
    );

    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    print_detect_summary(ctx, stats.ok, stats.miss, stats.invalid);

    if stats.invalid > 0 {
        Err(CliError::Message(i18n::tr("cli-detect-failed")))
    } else {
        Ok(())
    }
}

/// Internal struct.
struct DetectStats {
    /// Internal field.
    ok: usize,
    /// Internal field.
    miss: usize,
    /// Internal field.
    invalid: usize,
}

/// Internal struct.
struct FoundReport<'a> {
    /// Internal field.
    tag: &'a str,
    /// Internal field.
    identity: &'a str,
    /// Internal field.
    clone_check: &'a CloneCheck,
    /// Internal field.
    detect_score: Option<f32>,
    /// Internal field.
    decode_slot_hint: u8,
    /// Internal field.
    decode_slot_used: u8,
    /// Internal field.
    slot_status: &'a str,
    /// Internal field.
    slot_scan_count: u32,
}

/// Internal struct.
struct InvalidReport<'a> {
    /// Internal field.
    error: &'a str,
    /// Internal field.
    unverified: Option<&'a awmkit::Decoded>,
    /// Internal field.
    detect_score: Option<f32>,
    /// Internal field.
    decode_slot_hint: Option<u8>,
    /// Internal field.
    decode_slot_used: Option<u8>,
    /// Internal field.
    slot_status: &'a str,
    /// Internal field.
    slot_scan_count: u32,
}

/// Internal helper function.
fn log_parallelism(ctx: &Context) {
    if ctx.out.verbose() && !ctx.out.quiet() {
        let parallelism = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(1);
        let mut args = FluentArgs::new();
        args.set("workers", parallelism.to_string());
        ctx.out
            .info_diag(i18n::tr_args("cli-detect-parallelism-detail", &args));
    }
}

/// Internal helper function.
fn run_json_mode(
    inputs: &[std::path::PathBuf],
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
) -> Result<()> {
    let results: Vec<DetectJson> = inputs
        .iter()
        .map(|input| detect_one_json(audio, key_store, input, layout, evidence_store))
        .collect();
    let output = serde_json::to_string_pretty(&results)?;
    println!("{output}");
    Ok(())
}

/// Internal helper function.
fn build_progress(ctx: &Context, len: usize) -> Result<Option<ProgressBar>> {
    if ctx.out.quiet() {
        return Ok(None);
    }

    let bar = ProgressBar::new(len as u64);
    bar.set_style(
        ProgressStyle::with_template(DETECT_PROGRESS_TEMPLATE)
            .map_err(|e| CliError::Message(e.to_string()))?
            .progress_chars("=>-"),
    );
    bar.set_prefix("detect");
    Ok(Some(bar))
}

/// Internal helper function.
fn run_text_mode(
    ctx: &Context,
    inputs: &[std::path::PathBuf],
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
    progress: Option<&ProgressBar>,
) -> DetectStats {
    let mut stats = DetectStats {
        ok: 0,
        miss: 0,
        invalid: 0,
    };

    for input in inputs {
        let execution = detect_one(audio, key_store, input, layout, evidence_store);
        report_fallback_trace(ctx, progress, input, &execution);

        match execution.outcome {
            DetectOutcome::Found {
                tag,
                identity,
                version: _,
                key_slot: _,
                timestamp_minutes: _,
                timestamp_utc: _,
                pattern: _,
                bit_errors: _,
                match_found: _,
                clone_check,
                detect_score,
                decode_slot_hint,
                decode_slot_used,
                slot_status,
                slot_scan_count,
            } => {
                stats.ok += 1;
                report_found(
                    ctx,
                    progress,
                    input,
                    &FoundReport {
                        tag: &tag,
                        identity: &identity,
                        clone_check: &clone_check,
                        detect_score,
                        decode_slot_hint,
                        decode_slot_used,
                        slot_status: &slot_status,
                        slot_scan_count,
                    },
                );
            }
            DetectOutcome::NotFound => {
                stats.miss += 1;
                report_miss(ctx, progress, input);
            }
            DetectOutcome::Invalid {
                error,
                unverified,
                pattern: _,
                bit_errors: _,
                match_found: _,
                detect_score,
                decode_slot_hint,
                decode_slot_used,
                slot_status,
                slot_scan_count,
            } => {
                stats.invalid += 1;
                report_invalid(
                    ctx,
                    progress,
                    input,
                    &InvalidReport {
                        error: &error,
                        unverified: unverified.as_ref(),
                        detect_score,
                        decode_slot_hint,
                        decode_slot_used,
                        slot_status: &slot_status,
                        slot_scan_count,
                    },
                );
            }
            DetectOutcome::Error { error } => {
                stats.invalid += 1;
                report_error(ctx, progress, input, &error);
            }
        }

        if let Some(bar) = progress {
            bar.inc(1);
        }
    }

    stats
}

/// Internal helper function.
fn report_found(
    ctx: &Context,
    progress: Option<&ProgressBar>,
    input: &std::path::Path,
    report: &FoundReport<'_>,
) {
    if ctx.out.quiet() {
        return;
    }

    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    args.set("tag", report.tag);
    args.set("identity", report.identity);
    let line = i18n::tr_args("cli-detect-file-found", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        ctx.out.info_user(line);
    }

    if ctx.out.verbose() {
        let score_text = report
            .detect_score
            .map_or_else(|| "-".to_string(), |score| format!("{score:.3}"));
        let mut diag_args = FluentArgs::new();
        diag_args.set("path", input.display().to_string());
        diag_args.set("clone", report.clone_check.summary());
        diag_args.set("score", score_text);
        diag_args.set("slot_hint", report.decode_slot_hint.to_string());
        diag_args.set("slot_used", report.decode_slot_used.to_string());
        diag_args.set("slot_status", report.slot_status);
        diag_args.set("slot_scan_count", report.slot_scan_count.to_string());
        ctx.out
            .info_diag(i18n::tr_args("cli-detect-file-found-detail", &diag_args));
    }
}

/// Internal helper function.
fn report_miss(ctx: &Context, progress: Option<&ProgressBar>, input: &std::path::Path) {
    if ctx.out.quiet() {
        return;
    }
    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    let line = i18n::tr_args("cli-detect-file-miss", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        ctx.out.info_user(line);
    }
}

/// Internal helper function.
fn report_invalid(
    ctx: &Context,
    progress: Option<&ProgressBar>,
    input: &std::path::Path,
    report: &InvalidReport<'_>,
) {
    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    args.set("warning", i18n::tr("cli-detect-forensic-warning"));
    let line = i18n::tr_args("cli-detect-file-invalid", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        crate::output::Output::error_user(line);
    }

    if ctx.out.verbose() {
        let score_text = report
            .detect_score
            .map_or_else(|| "-".to_string(), |score| format!("{score:.3}"));
        let (tag, identity, timestamp, slot_from_unverified) = report.unverified.map_or(
            (
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ),
            |decoded| {
                (
                    decoded.tag.to_string(),
                    decoded.identity().to_string(),
                    decoded.timestamp_utc.to_string(),
                    decoded.key_slot.to_string(),
                )
            },
        );
        let mut detail_args = FluentArgs::new();
        detail_args.set("path", input.display().to_string());
        detail_args.set("error", report.error);
        detail_args.set("score", score_text);
        detail_args.set("tag", tag);
        detail_args.set("identity", identity);
        detail_args.set("timestamp", timestamp);
        detail_args.set("slot_unverified", slot_from_unverified);
        detail_args.set(
            "slot_hint",
            report
                .decode_slot_hint
                .map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        detail_args.set(
            "slot_used",
            report
                .decode_slot_used
                .map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        detail_args.set("slot_status", report.slot_status);
        detail_args.set("slot_scan_count", report.slot_scan_count.to_string());
        ctx.out.error_diag(i18n::tr_args(
            "cli-detect-file-invalid-detail",
            &detail_args,
        ));
    }
}

/// Internal helper function.
fn report_error(ctx: &Context, progress: Option<&ProgressBar>, input: &std::path::Path, err: &str) {
    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    let line = i18n::tr_args("cli-detect-file-error", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        crate::output::Output::error_user(line);
    }
    let mut detail_args = FluentArgs::new();
    detail_args.set("path", input.display().to_string());
    detail_args.set("error", err.to_string());
    ctx.out
        .error_diag(i18n::tr_args("cli-detect-file-error-detail", &detail_args));
}

/// Internal helper function.
fn report_fallback_trace(
    ctx: &Context,
    progress: Option<&ProgressBar>,
    input: &std::path::Path,
    execution: &DetectExecution,
) {
    if !execution.fallback_triggered || !ctx.out.verbose() || ctx.out.quiet() {
        return;
    }

    let reason = execution.fallback_reason.as_deref().unwrap_or("-");
    let outcome = match &execution.outcome {
        DetectOutcome::Error { .. } => "failed",
        _ => "ok",
    };
    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    args.set("route", execution.detect_route.as_str());
    args.set("reason", reason);
    args.set("outcome", outcome);
    let line = i18n::tr_args("cli-detect-fallback-detail", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        ctx.out.info_diag(line);
    }
}

/// Internal helper function.
fn print_detect_summary(ctx: &Context, ok: usize, miss: usize, invalid: usize) {
    if !ctx.out.quiet() {
        let mut args = FluentArgs::new();
        args.set("ok", ok.to_string());
        args.set("miss", miss.to_string());
        args.set("invalid", invalid.to_string());
        ctx.out.info_user(i18n::tr_args("cli-detect-done", &args));
    }
}

/// Internal enum.
enum DetectOutcome {
    /// Internal variant.
    Found {
        /// Internal field.
        tag: String,
        /// Internal field.
        identity: String,
        /// Internal field.
        version: u8,
        /// Internal field.
        key_slot: u8,
        /// Internal field.
        timestamp_minutes: u32,
        /// Internal field.
        timestamp_utc: u64,
        /// Internal field.
        pattern: String,
        /// Internal field.
        bit_errors: u32,
        /// Internal field.
        match_found: bool,
        /// Internal field.
        clone_check: CloneCheck,
        /// Internal field.
        detect_score: Option<f32>,
        /// Internal field.
        decode_slot_hint: u8,
        /// Internal field.
        decode_slot_used: u8,
        /// Internal field.
        slot_status: String,
        /// Internal field.
        slot_scan_count: u32,
    },
    /// Internal variant.
    NotFound,
    /// Internal variant.
    Invalid {
        /// Internal field.
        error: String,
        /// Internal field.
        unverified: Option<awmkit::Decoded>,
        /// Internal field.
        pattern: String,
        /// Internal field.
        bit_errors: u32,
        /// Internal field.
        match_found: bool,
        /// Internal field.
        detect_score: Option<f32>,
        /// Internal field.
        decode_slot_hint: Option<u8>,
        /// Internal field.
        decode_slot_used: Option<u8>,
        /// Internal field.
        slot_status: String,
        /// Internal field.
        slot_scan_count: u32,
    },
    /// Internal variant.
    Error {
        /// Internal field.
        error: String,
    },
}

/// Internal struct.
struct DetectExecution {
    /// Internal field.
    outcome: DetectOutcome,
    /// Internal field.
    detect_route: String,
    /// Internal field.
    fallback_triggered: bool,
    /// Internal field.
    fallback_reason: Option<String>,
}

#[derive(Clone)]
/// Internal struct.
struct CloneCheck {
    /// Internal field.
    check: String,
    /// Internal field.
    score: Option<f64>,
    /// Internal field.
    match_seconds: Option<f32>,
    /// Internal field.
    matched_evidence_id: Option<i64>,
    /// Internal field.
    reason: Option<String>,
}

impl CloneCheck {
    /// Internal associated function.
    fn exact(matched_evidence_id: i64) -> Self {
        Self {
            check: "exact".to_string(),
            score: None,
            match_seconds: None,
            matched_evidence_id: Some(matched_evidence_id),
            reason: None,
        }
    }

    /// Internal associated function.
    fn likely(matched_evidence_id: i64, score: f64, match_seconds: f32) -> Self {
        Self {
            check: "likely".to_string(),
            score: Some(score),
            match_seconds: Some(match_seconds),
            matched_evidence_id: Some(matched_evidence_id),
            reason: None,
        }
    }

    /// Internal associated function.
    fn suspect(score: Option<f64>, match_seconds: Option<f32>, reason: &str) -> Self {
        Self {
            check: "suspect".to_string(),
            score,
            match_seconds,
            matched_evidence_id: None,
            reason: Some(reason.to_string()),
        }
    }

    /// Internal associated function.
    fn unavailable(reason: String) -> Self {
        Self {
            check: "unavailable".to_string(),
            score: None,
            match_seconds: None,
            matched_evidence_id: None,
            reason: Some(reason),
        }
    }

    /// Internal helper method.
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
            "suspect" => self.reason.as_ref().map_or_else(
                || "suspect".to_string(),
                |reason| format!("suspect({reason})"),
            ),
            "unavailable" => self.reason.as_ref().map_or_else(
                || "unavailable".to_string(),
                |reason| format!("unavailable({reason})"),
            ),
            other => other.to_string(),
        }
    }
}

/// Internal helper function.
fn detect_one(
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    input: &std::path::Path,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
) -> DetectExecution {
    let mut detect_route = "multichannel".to_string();
    let mut fallback_triggered = false;
    let mut fallback_reason: Option<String> = None;

    let best_result = match audio.detect_multichannel(input, layout) {
        Ok(result) => result.best,
        Err(err) if is_strict_adm_fallback_error(&err) => {
            detect_route = "single_fallback".to_string();
            fallback_triggered = true;
            fallback_reason = Some(err.to_string());
            match audio.detect(input) {
                Ok(result) => result,
                Err(fallback_err) => {
                    return DetectExecution {
                        outcome: DetectOutcome::Error {
                            error: format!("fallback detect failed: {fallback_err}"),
                        },
                        detect_route,
                        fallback_triggered,
                        fallback_reason,
                    };
                }
            }
        }
        Err(err) => {
            return DetectExecution {
                outcome: DetectOutcome::Error {
                    error: err.to_string(),
                },
                detect_route,
                fallback_triggered,
                fallback_reason,
            };
        }
    };

    let outcome = match best_result {
        None => DetectOutcome::NotFound,
        Some(result) => match resolve_decode_slot(&result.raw_message, key_store) {
            SlotResolution::Decoded(decoded) => {
                let clone_check = evaluate_clone_check(input, &decoded.message, evidence_store);
                DetectOutcome::Found {
                    tag: decoded.message.tag.to_string(),
                    identity: decoded.message.identity().to_string(),
                    version: decoded.message.version,
                    key_slot: decoded.message.key_slot,
                    timestamp_minutes: decoded.message.timestamp_minutes,
                    timestamp_utc: decoded.message.timestamp_utc,
                    pattern: result.pattern,
                    bit_errors: result.bit_errors,
                    match_found: result.match_found,
                    clone_check,
                    detect_score: result.detect_score,
                    decode_slot_hint: decoded.slot_hint,
                    decode_slot_used: decoded.slot_used,
                    slot_status: decoded.status,
                    slot_scan_count: decoded.scan_count,
                }
            }
            SlotResolution::Invalid(invalid) => DetectOutcome::Invalid {
                error: invalid.error,
                unverified: Message::decode_unverified(&result.raw_message).ok(),
                pattern: result.pattern,
                bit_errors: result.bit_errors,
                match_found: result.match_found,
                detect_score: result.detect_score,
                decode_slot_hint: Some(invalid.slot_hint),
                decode_slot_used: invalid.slot_used,
                slot_status: invalid.status,
                slot_scan_count: invalid.scan_count,
            },
        },
    };

    DetectExecution {
        outcome,
        detect_route,
        fallback_triggered,
        fallback_reason,
    }
}

/// Internal helper function.
fn detect_one_json(
    audio: &awmkit::Audio,
    key_store: &KeyStore,
    input: &std::path::Path,
    layout: Option<ChannelLayout>,
    evidence_store: Option<&EvidenceStore>,
) -> DetectJson {
    let execution = detect_one(audio, key_store, input, layout, evidence_store);
    let DetectExecution {
        outcome,
        detect_route,
        fallback_triggered,
        fallback_reason,
    } = execution;
    let mut json = match outcome {
        DetectOutcome::NotFound => detect_json_base(input, "not_found"),
        DetectOutcome::Found {
            tag,
            identity,
            version,
            key_slot,
            timestamp_minutes,
            timestamp_utc,
            pattern,
            bit_errors,
            match_found,
            clone_check,
            detect_score,
            decode_slot_hint,
            decode_slot_used,
            slot_status,
            slot_scan_count,
        } => {
            let mut json = detect_json_base(input, "ok");
            json.verification = Some("verified".to_string());
            json.tag = Some(tag);
            json.identity = Some(identity);
            json.version = Some(version);
            json.key_slot = Some(key_slot);
            json.timestamp_minutes = Some(timestamp_minutes);
            json.timestamp_utc = Some(timestamp_utc);
            json.pattern = Some(pattern);
            json.detect_score = detect_score;
            json.bit_errors = Some(bit_errors);
            json.match_found = Some(match_found);
            json.clone_check = Some(clone_check.check);
            json.clone_score = clone_check.score;
            json.clone_match_seconds = clone_check.match_seconds;
            json.clone_matched_evidence_id = clone_check.matched_evidence_id;
            json.clone_reason = clone_check.reason;
            json.decode_slot_hint = Some(decode_slot_hint);
            json.decode_slot_used = Some(decode_slot_used);
            json.slot_status = Some(slot_status);
            json.slot_scan_count = Some(slot_scan_count);
            json
        }
        DetectOutcome::Invalid {
            error,
            unverified,
            pattern,
            bit_errors,
            match_found,
            detect_score,
            decode_slot_hint,
            decode_slot_used,
            slot_status,
            slot_scan_count,
        } => {
            let mut json = detect_json_base(input, "invalid_hmac");
            json.verification = Some("unverified".to_string());
            json.forensic_warning = Some(i18n::tr("cli-detect-forensic-warning"));
            json.tag = unverified.as_ref().map(|message| message.tag.to_string());
            json.identity = unverified
                .as_ref()
                .map(|message| message.identity().to_string());
            json.version = unverified.as_ref().map(|message| message.version);
            json.key_slot = unverified.as_ref().map(|message| message.key_slot);
            json.timestamp_minutes = unverified.as_ref().map(|message| message.timestamp_minutes);
            json.timestamp_utc = unverified.as_ref().map(|message| message.timestamp_utc);
            json.pattern = Some(pattern);
            json.detect_score = detect_score;
            json.bit_errors = Some(bit_errors);
            json.match_found = Some(match_found);
            json.error = Some(error);
            json.decode_slot_hint = decode_slot_hint;
            json.decode_slot_used = decode_slot_used;
            json.slot_status = Some(slot_status);
            json.slot_scan_count = Some(slot_scan_count);
            json
        }
        DetectOutcome::Error { error } => detect_json_error(input, error),
    };
    json.detect_route = detect_route;
    json.fallback_triggered = fallback_triggered;
    json.fallback_reason = fallback_reason;
    json
}

/// Internal helper function.
fn detect_json_base(input: &std::path::Path, status: &str) -> DetectJson {
    DetectJson {
        file: input.display().to_string(),
        status: status.to_string(),
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
        detect_route: "multichannel".to_string(),
        fallback_triggered: false,
        fallback_reason: None,
    }
}

/// Internal helper function.
fn detect_json_error(input: &std::path::Path, error: String) -> DetectJson {
    let mut json = detect_json_base(input, "error");
    json.error = Some(error);
    json
}

/// Internal helper function.
const fn is_strict_adm_fallback_error(err: &awmkit::Error) -> bool {
    matches!(
        err,
        awmkit::Error::AdmUnsupported(_)
            | awmkit::Error::AdmPreserveFailed(_)
            | awmkit::Error::AdmPcmFormatUnsupported(_)
    )
}

/// Internal struct.
struct DecodedSlotMessage {
    /// Internal field.
    message: awmkit::Decoded,
    /// Internal field.
    slot_hint: u8,
    /// Internal field.
    slot_used: u8,
    /// Internal field.
    status: String,
    /// Internal field.
    scan_count: u32,
}

/// Internal struct.
struct InvalidSlotDecode {
    /// Internal field.
    slot_hint: u8,
    /// Internal field.
    slot_used: Option<u8>,
    /// Internal field.
    status: String,
    /// Internal field.
    scan_count: u32,
    /// Internal field.
    error: String,
}

/// Internal enum.
enum SlotResolution {
    /// Internal variant.
    Decoded(DecodedSlotMessage),
    /// Internal variant.
    Invalid(InvalidSlotDecode),
}

/// Internal helper function.
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

    let mut decode_successes: Vec<(u8, awmkit::Decoded)> = Vec::new();
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
            Err(Failure::KeyNotFound) => {
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

/// Internal helper function.
fn evaluate_clone_check(
    input: &std::path::Path,
    decoded: &awmkit::Decoded,
    evidence_store: Option<&EvidenceStore>,
) -> CloneCheck {
    let Some(evidence_store) = evidence_store else {
        return CloneCheck::unavailable("evidence_store_unavailable".to_string());
    };

    let proof = match build_proof(input) {
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

/// Internal helper function.
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
