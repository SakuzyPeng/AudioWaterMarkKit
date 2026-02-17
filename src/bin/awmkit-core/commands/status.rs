use crate::error::{CliError, Result};
use crate::util::audio_from_context;
use crate::Context;
use awmkit::app::{i18n, EvidenceStore, KeyStore, TagStore, KEY_LEN};
use clap::Args;
use fluent_bundle::FluentArgs;

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// Run extended diagnostics.
    #[arg(long)]
    pub doctor: bool,
}

/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    print_version(ctx);
    let store = KeyStore::new()?;
    print_active_slot(ctx, store.active_slot()?);
    print_key_status(ctx, &store)?;

    match audio_from_context(ctx) {
        Ok(audio) => {
            if args.doctor {
                run_doctor(ctx, &audio);
            } else {
                ctx.out.info(i18n::tr("cli-status-audiowmark_found"));
            }
        }
        Err(CliError::AudiowmarkNotFound) => {
            ctx.out.warn(i18n::tr("cli-status-audiowmark_not_found"));
        }
        Err(err) => return Err(err),
    }

    Ok(())
}

/// Internal helper function.
fn print_version(ctx: &Context) {
    let mut fmt_args = FluentArgs::new();
    fmt_args.set("version", env!("CARGO_PKG_VERSION"));
    ctx.out.info(i18n::tr_args("cli-status-version", &fmt_args));
}

/// Internal helper function.
fn print_active_slot(ctx: &Context, active_slot: u8) {
    let mut slot_args = FluentArgs::new();
    slot_args.set("slot", active_slot.to_string());
    ctx.out
        .info(i18n::tr_args("cli-key-slot-current", &slot_args));
}

/// Internal helper function.
fn print_key_status(ctx: &Context, store: &KeyStore) -> Result<()> {
    if store.exists() {
        let (key, backend) = store.load_with_backend()?;
        let mut key_args = FluentArgs::new();
        key_args.set("bytes", key.len().to_string());
        ctx.out
            .info(i18n::tr_args("cli-status-key_configured", &key_args));

        let mut backend_args = FluentArgs::new();
        backend_args.set("backend", backend.label());
        ctx.out
            .info(i18n::tr_args("cli-status-key_storage", &backend_args));

        if key.len() != KEY_LEN {
            ctx.out.warn(i18n::tr("cli-status-key_len_mismatch"));
        }
    } else {
        ctx.out.info(i18n::tr("cli-status-key_not_configured"));
    }
    Ok(())
}

/// Internal helper function.
fn run_doctor(ctx: &Context, audio: &awmkit::Audio) {
    print_audio_health(ctx, audio);
    print_audio_capabilities(ctx, audio);
    print_db_status(ctx);
}

/// Internal helper function.
fn print_audio_health(ctx: &Context, audio: &awmkit::Audio) {
    if audio.is_available() {
        ctx.out.info(i18n::tr("cli-status-audiowmark_available"));
    } else {
        ctx.out
            .warn(i18n::tr("cli-status-audiowmark_not_responding"));
    }

    match audio.version() {
        Ok(version) => {
            let mut fmt_args = FluentArgs::new();
            fmt_args.set("version", version.as_str());
            ctx.out
                .info(i18n::tr_args("cli-status-audiowmark_version", &fmt_args));
        }
        Err(err) => {
            let mut fmt_args = FluentArgs::new();
            fmt_args.set("error", err.to_string());
            ctx.out.warn(i18n::tr_args(
                "cli-status-audiowmark_version_error",
                &fmt_args,
            ));
        }
    }

    let mut path_args = FluentArgs::new();
    path_args.set("path", audio.binary_path().display().to_string());
    ctx.out
        .info(i18n::tr_args("cli-status-audiowmark_path", &path_args));
}

/// Internal helper function.
fn print_audio_capabilities(ctx: &Context, audio: &awmkit::Audio) {
    let caps = audio.media_capabilities();

    let mut backend_args = FluentArgs::new();
    backend_args.set("backend", caps.backend);
    ctx.out
        .info(i18n::tr_args("cli-status-media_backend", &backend_args));

    let mut eac3_args = FluentArgs::new();
    eac3_args.set(
        "available",
        if caps.eac3_decode {
            "available"
        } else {
            "unavailable"
        },
    );
    ctx.out
        .info(i18n::tr_args("cli-status-media-eac3", &eac3_args));

    let mut containers_args = FluentArgs::new();
    containers_args.set("containers", caps.supported_containers_csv());
    ctx.out.info(i18n::tr_args(
        "cli-status-media-containers",
        &containers_args,
    ));

    let mut policy_args = FluentArgs::new();
    policy_args.set("input_policy", i18n::tr("cli-status-media-policy-input"));
    policy_args.set("output_policy", i18n::tr("cli-status-media-policy-output"));
    ctx.out
        .info(i18n::tr_args("cli-status-media-policy", &policy_args));
}

/// Internal helper function.
fn print_db_status(ctx: &Context) {
    match TagStore::load() {
        Ok(tags) => {
            ctx.out.info(format!("db.mappings={}", tags.list().len()));
        }
        Err(err) => {
            ctx.out.warn(format!("db.mappings=unavailable ({err})"));
        }
    }

    match EvidenceStore::load() {
        Ok(evidence_store) => match evidence_store.count_all() {
            Ok(count) => ctx.out.info(format!("db.evidence={count}")),
            Err(err) => ctx.out.warn(format!("db.evidence=unavailable ({err})")),
        },
        Err(err) => {
            ctx.out.warn(format!("db.evidence=unavailable ({err})"));
        }
    }
}
