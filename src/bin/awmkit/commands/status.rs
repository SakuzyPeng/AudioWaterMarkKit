use crate::error::{CliError, Result};
use crate::util::audio_from_context;
use crate::Context;
use awmkit::app::{i18n, KeyStore, KEY_LEN};
use clap::Args;
use fluent_bundle::FluentArgs;

#[derive(Args)]
pub struct StatusArgs {
    /// Run extended diagnostics
    #[arg(long)]
    pub doctor: bool,
}

pub fn run(ctx: &Context, args: &StatusArgs) -> Result<()> {
    let mut fmt_args = FluentArgs::new();
    fmt_args.set("version", env!("CARGO_PKG_VERSION"));
    ctx.out.info(i18n::tr_args("cli-status-version", &fmt_args));

    let store = KeyStore::new()?;
    if store.exists() {
        let (key, backend) = store.load_with_backend()?;
        let mut fmt_args = FluentArgs::new();
        fmt_args.set("bytes", key.len().to_string());
        ctx.out.info(i18n::tr_args("cli-status-key_configured", &fmt_args));
        let mut fmt_args = FluentArgs::new();
        fmt_args.set("backend", backend.label());
        ctx.out.info(i18n::tr_args("cli-status-key_storage", &fmt_args));
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
                    ctx.out.warn(i18n::tr("cli-status-audiowmark_not_responding"));
                }
                match audio.version() {
                    Ok(version) => {
                        let mut fmt_args = FluentArgs::new();
                        fmt_args.set("version", version.as_str());
                        ctx.out.info(i18n::tr_args("cli-status-audiowmark_version", &fmt_args));
                    }
                    Err(err) => {
                        let mut fmt_args = FluentArgs::new();
                        fmt_args.set("error", err.to_string());
                        ctx.out.warn(i18n::tr_args("cli-status-audiowmark_version_error", &fmt_args));
                    }
                }
                let mut fmt_args = FluentArgs::new();
                fmt_args.set("path", audio.binary_path().display().to_string());
                ctx.out.info(i18n::tr_args("cli-status-audiowmark_path", &fmt_args));
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
