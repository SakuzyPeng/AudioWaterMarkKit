use crate::error::{CliError, Result};
use crate::keystore::{KeyStore, KEY_LEN};
use crate::util::audio_from_context;
use crate::Context;
use clap::Args;

#[derive(Args)]
pub struct StatusArgs {
    /// Run extended diagnostics
    #[arg(long)]
    pub doctor: bool,
}

pub fn run(ctx: &Context, args: &StatusArgs) -> Result<()> {
    ctx.out
        .info(format!("awmkit v{}", env!("CARGO_PKG_VERSION")));

    let store = KeyStore::new()?;
    if store.exists() {
        let key = store.load()?;
        ctx.out
            .info(format!("Key: configured ({} bytes)", key.len()));
        if key.len() != KEY_LEN {
            ctx.out.warn("Key length does not match expected size");
        }
    } else {
        ctx.out.info("Key: not configured");
    }

    match audio_from_context(ctx) {
        Ok(audio) => {
            if args.doctor {
                if audio.is_available() {
                    ctx.out.info("audiowmark: available");
                } else {
                    ctx.out.warn("audiowmark: not responding");
                }
                match audio.version() {
                    Ok(version) => ctx.out.info(format!("audiowmark version: {version}")),
                    Err(err) => ctx.out.warn(format!("audiowmark version error: {err}")),
                }
                ctx.out
                    .info(format!("audiowmark path: {}", audio.binary_path().display()));
            } else {
                ctx.out.info("audiowmark: found");
            }
        }
        Err(CliError::AudiowmarkNotFound) => {
            ctx.out.warn("audiowmark: not found");
        }
        Err(err) => return Err(err),
    }

    Ok(())
}
