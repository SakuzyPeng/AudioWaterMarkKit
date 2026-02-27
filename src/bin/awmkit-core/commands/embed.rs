use crate::error::{CliError, Result};
use crate::util::{
    audio_from_context, default_output_path, ensure_file, expand_inputs, parse_tag, CliLayout,
};
use crate::Context;
use awmkit::app::{
    analyze, build_proof, i18n, key_id_from_key_material, Analysis, EvidenceStore, KeyStore,
    NewAudioEvidence, TagStore, SNR_STATUS_OK,
};
use awmkit::{Error as AwmError, Message};
use clap::Args;
use fluent_bundle::FluentArgs;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

/// Internal constant.
const EMBED_PROGRESS_TEMPLATE: &str = "{prefix} [{bar:40}] {pos}/{len}";

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// Tag (1-7 identity or full 8-char tag).
    #[arg(long)]
    pub tag: String,

    /// Watermark strength (1-30).
    #[arg(long, default_value_t = 10)]
    pub strength: u8,

    /// Channel layout (default: auto).
    #[arg(long, value_enum, default_value_t = CliLayout::Auto)]
    pub layout: CliLayout,

    /// Output file path (single input only).
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Input files (supports glob).
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    let inputs = expand_inputs(&args.inputs)?;
    if args.output.is_some() && inputs.len() != 1 {
        return Err(CliError::Message(i18n::tr("cli-embed-output_single")));
    }

    for input in &inputs {
        ensure_file(input)?;
    }

    let store = KeyStore::new()?;
    let active_slot = store.active_slot()?;
    let key = store.load_slot(active_slot)?;
    let tag = parse_tag(&args.tag)?;
    let message = Message::encode_with_slot(awmkit::CURRENT_VERSION, &tag, &key, active_slot)?;
    let decoded_message = Message::decode(&message, &key)?;
    let evidence_store = match EvidenceStore::load() {
        Ok(store) => Some(store),
        Err(err) => {
            let mut args_i18n = FluentArgs::new();
            args_i18n.set("error", err.to_string());
            ctx.out.warn_diag(i18n::tr_args(
                "cli-embed-evidence-store-unavailable-detail",
                &args_i18n,
            ));
            None
        }
    };

    let audio = audio_from_context(ctx)?.strength(args.strength);
    let layout = args.layout.to_channel_layout();

    let progress = build_progress(ctx, inputs.len())?;
    let mut stats = EmbedStats::default();
    print_embed_intro(ctx);
    let shared = EmbedShared {
        ctx,
        audio: &audio,
        layout,
        message: &message,
        decoded_message: &decoded_message,
        key: &key,
        evidence_store: evidence_store.as_ref(),
        progress: progress.as_ref(),
    };

    for input in inputs {
        let output = resolve_output_path(args.output.as_ref(), &input)?;
        process_embed_input(&shared, &input, &output, &mut stats);
    }

    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    print_embed_summary(ctx, &stats);
    save_identity_mapping(ctx, &stats, &decoded_message, &tag);

    if stats.failed > 0 {
        Err(CliError::Message(i18n::tr("cli-embed-failed")))
    } else {
        Ok(())
    }
}

#[derive(Default)]
/// Internal struct.
struct EmbedStats {
    /// Internal field.
    success: usize,
    /// Internal field.
    failed: usize,
    /// Internal field.
    skipped: usize,
    /// Internal field.
    failure_details: Vec<String>,
}

/// Internal struct.
struct EmbedShared<'a> {
    /// Internal field.
    ctx: &'a Context,
    /// Internal field.
    audio: &'a awmkit::Audio,
    /// Internal field.
    layout: Option<awmkit::ChannelLayout>,
    /// Internal field.
    message: &'a [u8; awmkit::MESSAGE_LEN],
    /// Internal field.
    decoded_message: &'a awmkit::Decoded,
    /// Internal field.
    key: &'a [u8],
    /// Internal field.
    evidence_store: Option<&'a EvidenceStore>,
    /// Internal field.
    progress: Option<&'a ProgressBar>,
}

/// Internal helper function.
fn build_progress(ctx: &Context, len: usize) -> Result<Option<ProgressBar>> {
    if ctx.out.quiet() {
        return Ok(None);
    }
    let bar = ProgressBar::new(len as u64);
    bar.set_style(
        ProgressStyle::with_template(EMBED_PROGRESS_TEMPLATE)
            .map_err(|e| CliError::Message(e.to_string()))?
            .progress_chars("=>-"),
    );
    bar.set_prefix("embed");
    Ok(Some(bar))
}

/// Internal helper function.
fn print_embed_intro(ctx: &Context) {
    ctx.out
        .info_diag(i18n::tr("cli-embed-intro-routing-detail"));
    if ctx.out.verbose() {
        let parallelism = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(1);
        let mut args = FluentArgs::new();
        args.set("workers", parallelism.to_string());
        ctx.out
            .info_diag(i18n::tr_args("cli-embed-intro-parallelism-detail", &args));
    }
}

/// Internal helper function.
fn resolve_output_path(output_arg: Option<&PathBuf>, input: &std::path::Path) -> Result<PathBuf> {
    output_arg.map_or_else(|| default_output_path(input), |path| Ok(path.clone()))
}

/// Internal helper function.
fn process_embed_input(
    shared: &EmbedShared<'_>,
    input: &std::path::Path,
    output: &std::path::Path,
    stats: &mut EmbedStats,
) {
    if !handle_precheck(shared, input, stats) {
        return;
    }

    match shared
        .audio
        .embed_multichannel(input, output, shared.message, shared.layout)
    {
        Ok(()) => {
            stats.success = stats.success.saturating_add(1);
            let snr = analyze(input, output);
            persist_evidence(shared, input, output, &snr);
            report_embed_ok(shared.ctx, input, output, &snr);
        }
        Err(err) => {
            stats.failed = stats.failed.saturating_add(1);
            stats
                .failure_details
                .push(format!("{}: {err}", input.display()));
            report_embed_error(shared.ctx, shared.progress, input, &err.to_string());
        }
    }

    if let Some(bar) = shared.progress {
        bar.inc(1);
    }
}

/// Internal helper function.
fn handle_precheck(
    shared: &EmbedShared<'_>,
    input: &std::path::Path,
    stats: &mut EmbedStats,
) -> bool {
    match shared.audio.detect_multichannel(input, shared.layout) {
        Ok(detect) => {
            if detect.best.is_some() {
                stats.skipped = stats.skipped.saturating_add(1);
                let mut args = FluentArgs::new();
                args.set("path", input.display().to_string());
                let line = i18n::tr_args("cli-embed-skip-existing", &args);
                if let Some(bar) = shared.progress {
                    bar.println(line);
                    bar.inc(1);
                } else if !shared.ctx.out.quiet() {
                    shared.ctx.out.warn_user(line);
                }
                return false;
            }
        }
        Err(err) => {
            if matches!(err, AwmError::AdmUnsupported(_)) {
                let mut args = FluentArgs::new();
                args.set("path", input.display().to_string());
                let line = i18n::tr_args("cli-embed-precheck-adm-fallback", &args);
                if let Some(bar) = shared.progress {
                    bar.println(line);
                } else if !shared.ctx.out.quiet() {
                    shared.ctx.out.warn_user(line);
                }
                let mut detail_args = FluentArgs::new();
                detail_args.set("path", input.display().to_string());
                detail_args.set("error", err.to_string());
                shared.ctx.out.warn_diag(i18n::tr_args(
                    "cli-embed-precheck-adm-fallback-detail",
                    &detail_args,
                ));
            } else {
                stats.failed = stats.failed.saturating_add(1);
                report_embed_error(shared.ctx, shared.progress, input, &err.to_string());
                if let Some(bar) = shared.progress {
                    bar.inc(1);
                }
                return false;
            }
        }
    }
    true
}

/// Internal helper function.
fn persist_evidence(
    shared: &EmbedShared<'_>,
    input: &std::path::Path,
    output: &std::path::Path,
    snr: &Analysis,
) {
    let Some(evidence_store) = shared.evidence_store else {
        return;
    };

    let proof = match build_proof(output) {
        Ok(proof) => proof,
        Err(err) => {
            let mut args = FluentArgs::new();
            args.set("input", input.display().to_string());
            args.set("output", output.display().to_string());
            args.set("error", err.to_string());
            shared.ctx.out.warn_diag(i18n::tr_args(
                "cli-embed-evidence-proof-failed-detail",
                &args,
            ));
            return;
        }
    };

    let insert = NewAudioEvidence {
        file_path: output.display().to_string(),
        tag: shared.decoded_message.tag.to_string(),
        identity: shared.decoded_message.identity().to_string(),
        version: shared.decoded_message.version,
        key_slot: shared.decoded_message.key_slot,
        timestamp_minutes: shared.decoded_message.timestamp_minutes,
        message_hex: hex::encode(shared.message),
        sample_rate: proof.sample_rate,
        channels: proof.channels,
        sample_count: proof.sample_count,
        pcm_sha256: proof.pcm_sha256,
        key_id: key_id_from_key_material(shared.key),
        is_forced_embed: false,
        snr_db: snr.snr_db,
        snr_status: snr.status.clone(),
        chromaprint: proof.chromaprint,
        fp_config_id: proof.fp_config_id,
    };
    if let Err(err) = evidence_store.insert(&insert) {
        let mut args = FluentArgs::new();
        args.set("input", input.display().to_string());
        args.set("output", output.display().to_string());
        args.set("error", err.to_string());
        shared.ctx.out.warn_diag(i18n::tr_args(
            "cli-embed-evidence-insert-failed-detail",
            &args,
        ));
    }
}

/// Internal helper function.
fn report_embed_ok(
    ctx: &Context,
    input: &std::path::Path,
    output: &std::path::Path,
    snr: &Analysis,
) {
    if ctx.out.quiet() {
        return;
    }
    if snr.status == SNR_STATUS_OK {
        let mut args = FluentArgs::new();
        args.set("input", input.display().to_string());
        args.set("output", output.display().to_string());
        args.set("snr", format!("{:.2}", snr.snr_db.unwrap_or_default()));
        ctx.out
            .info_user(i18n::tr_args("cli-embed-file-ok-snr", &args));
    } else {
        let mut args = FluentArgs::new();
        args.set("input", input.display().to_string());
        args.set("output", output.display().to_string());
        ctx.out.info_user(i18n::tr_args("cli-embed-file-ok", &args));
        let reason = snr.detail.clone().unwrap_or_else(|| snr.status.clone());
        let mut detail_args = FluentArgs::new();
        detail_args.set("input", input.display().to_string());
        detail_args.set("output", output.display().to_string());
        detail_args.set("reason", reason);
        ctx.out.info_diag(i18n::tr_args(
            "cli-embed-snr-unavailable-detail",
            &detail_args,
        ));
    }
}

/// Internal helper function.
fn report_embed_error(
    ctx: &Context,
    progress: Option<&ProgressBar>,
    input: &std::path::Path,
    err: &str,
) {
    let mut args = FluentArgs::new();
    args.set("path", input.display().to_string());
    let line = i18n::tr_args("cli-embed-file-failed", &args);
    if let Some(bar) = progress {
        bar.println(line);
    } else {
        crate::output::Output::error_user(line);
    }
    let mut detail_args = FluentArgs::new();
    detail_args.set("path", input.display().to_string());
    detail_args.set("error", err.to_string());
    ctx.out
        .error_diag(i18n::tr_args("cli-embed-file-failed-detail", &detail_args));
}

/// Internal helper function.
fn print_embed_summary(ctx: &Context, stats: &EmbedStats) {
    if ctx.out.quiet() {
        return;
    }
    let mut args = FluentArgs::new();
    args.set("success", stats.success.to_string());
    args.set("failed", stats.failed.to_string());
    ctx.out.info_user(i18n::tr_args("cli-embed-done", &args));
    if stats.skipped > 0 {
        let mut skipped_args = FluentArgs::new();
        skipped_args.set("count", stats.skipped.to_string());
        ctx.out
            .warn_user(i18n::tr_args("cli-embed-skipped-count", &skipped_args));
    }
    if ctx.out.verbose() && !stats.failure_details.is_empty() {
        ctx.out
            .warn_diag(i18n::tr("cli-embed-failure-details-title-detail"));
        for detail in stats.failure_details.iter().take(8) {
            let mut detail_args = FluentArgs::new();
            detail_args.set("detail", detail.as_str());
            ctx.out.warn_diag(i18n::tr_args(
                "cli-embed-failure-details-item-detail",
                &detail_args,
            ));
        }
        let remain = stats.failure_details.len().saturating_sub(8);
        if remain > 0 {
            let mut omit_args = FluentArgs::new();
            omit_args.set("count", remain.to_string());
            ctx.out.warn_diag(i18n::tr_args(
                "cli-embed-failure-details-omitted-detail",
                &omit_args,
            ));
        }
    }
}

/// Internal helper function.
fn save_identity_mapping(
    ctx: &Context,
    stats: &EmbedStats,
    decoded_message: &awmkit::Decoded,
    tag: &awmkit::Tag,
) {
    if stats.success == 0 {
        return;
    }
    match TagStore::load() {
        Ok(mut store) => match store.save_if_absent(decoded_message.identity(), tag) {
            Ok(inserted) if inserted && !ctx.out.quiet() => {
                let mut args = FluentArgs::new();
                args.set("identity", decoded_message.identity().to_string());
                args.set("tag", decoded_message.tag.to_string());
                ctx.out
                    .info_user(i18n::tr_args("cli-embed-mapping-autosaved", &args));
            }
            Ok(_) => {}
            Err(err) => {
                let mut args = FluentArgs::new();
                args.set("error", err.to_string());
                ctx.out
                    .warn_diag(i18n::tr_args("cli-embed-mapping-save-failed-detail", &args));
            }
        },
        Err(err) => {
            let mut args = FluentArgs::new();
            args.set("error", err.to_string());
            ctx.out
                .warn_diag(i18n::tr_args("cli-embed-mapping-load-failed-detail", &args));
        }
    }
}
