use crate::error::{CliError, Result};
use crate::util::audio_from_context;
use crate::Context;
use awmkit::app::{i18n, EvidenceStore, KeyStore, TagStore, KEY_LEN};
use clap::Args;
use fluent_bundle::FluentArgs;

#[derive(Args)]
pub struct CmdArgs {
    /// Run extended diagnostics
    #[arg(long)]
    pub doctor: bool,
}

#[allow(clippy::too_many_lines)]
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    let mut fmt_args = FluentArgs::new();
    fmt_args.set("version", env!("CARGO_PKG_VERSION"));
    ctx.out.info(i18n::tr_args("cli-status-version", &fmt_args));

    let store = KeyStore::new()?;
    let active_slot = store.active_slot()?;
    let mut slot_args = FluentArgs::new();
    slot_args.set("slot", active_slot.to_string());
    ctx.out
        .info(i18n::tr_args("cli-key-slot-current", &slot_args));

    if store.exists() {
        let (key, backend) = store.load_with_backend()?;
        let mut fmt_args = FluentArgs::new();
        fmt_args.set("bytes", key.len().to_string());
        ctx.out
            .info(i18n::tr_args("cli-status-key_configured", &fmt_args));
        let mut fmt_args = FluentArgs::new();
        fmt_args.set("backend", backend.label());
        ctx.out
            .info(i18n::tr_args("cli-status-key_storage", &fmt_args));
        if key.len() != KEY_LEN {
            ctx.out.warn(i18n::tr("cli-status-key_len_mismatch"));
        }
    } else {
        ctx.out.info(i18n::tr("cli-status-key_not_configured"));
    }

    match audio_from_context(ctx) {
        Ok(audio) => {
            if args.doctor {
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
                let mut fmt_args = FluentArgs::new();
                fmt_args.set("path", audio.binary_path().display().to_string());
                ctx.out
                    .info(i18n::tr_args("cli-status-audiowmark_path", &fmt_args));

                let caps = audio.media_capabilities();
                let mut fmt_args = FluentArgs::new();
                fmt_args.set("backend", caps.backend);
                ctx.out
                    .info(i18n::tr_args("cli-status-media_backend", &fmt_args));
                let mut fmt_args = FluentArgs::new();
                fmt_args.set(
                    "available",
                    if caps.eac3_decode {
                        "available"
                    } else {
                        "unavailable"
                    },
                );
                ctx.out
                    .info(i18n::tr_args("cli-status-media-eac3", &fmt_args));
                let mut fmt_args = FluentArgs::new();
                fmt_args.set("containers", caps.supported_containers_csv());
                ctx.out
                    .info(i18n::tr_args("cli-status-media-containers", &fmt_args));
                let mut fmt_args = FluentArgs::new();
                let input_policy = i18n::tr("cli-status-media-policy-input");
                let output_policy = i18n::tr("cli-status-media-policy-output");
                fmt_args.set("input_policy", input_policy);
                fmt_args.set("output_policy", output_policy);
                ctx.out
                    .info(i18n::tr_args("cli-status-media-policy", &fmt_args));

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
