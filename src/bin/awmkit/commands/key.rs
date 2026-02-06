use crate::error::{CliError, Result};
use crate::Context;
use crate::KeyCommand;
use awmkit::app::{generate_key, i18n, KeyStore, KEY_LEN};
use clap::Args;
use fluent_bundle::FluentArgs;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Args)]
pub struct ImportArgs {
    /// Key file path (binary)
    pub file: PathBuf,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file path (binary)
    pub file: PathBuf,

    /// Overwrite if exists
    #[arg(long)]
    pub force: bool,
}

pub fn run(ctx: &Context, command: KeyCommand) -> Result<()> {
    match command {
        KeyCommand::Show => show(ctx),
        KeyCommand::Import(args) => import(ctx, &args),
        KeyCommand::Export(args) => export(ctx, &args),
        KeyCommand::Rotate => rotate(ctx),
    }
}

fn show(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    let (key, backend) = store.load_with_backend()?;

    let mut hasher = Sha256::new();
    hasher.update(&key);
    let fingerprint = hex::encode(hasher.finalize());

    ctx.out.info(i18n::tr("cli-key-status_configured"));
    let mut args = FluentArgs::new();
    args.set("bytes", KEY_LEN.to_string());
    ctx.out.info(i18n::tr_args("cli-key-length", &args));
    let mut args = FluentArgs::new();
    args.set("fingerprint", fingerprint.as_str());
    ctx.out.info(i18n::tr_args("cli-key-fingerprint", &args));
    let mut args = FluentArgs::new();
    args.set("backend", backend.label());
    ctx.out.info(i18n::tr_args("cli-key-storage", &args));
    Ok(())
}

fn import(ctx: &Context, args: &ImportArgs) -> Result<()> {
    let bytes = std::fs::read(&args.file)?;
    if bytes.len() != KEY_LEN {
        return Err(CliError::InvalidKeyLength {
            expected: KEY_LEN,
            actual: bytes.len(),
        });
    }

    let store = KeyStore::new()?;
    let existed = store.exists();
    store.save(&bytes)?;

    if existed {
        ctx.out.info(i18n::tr("cli-key-replaced"));
    } else {
        ctx.out.info(i18n::tr("cli-key-imported"));
    }
    Ok(())
}

fn export(ctx: &Context, args: &ExportArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let key = store.load()?;

    let file = if args.force {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&args.file)?
    } else {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.file)?
    };

    let mut file = file;
    file.write_all(&key)?;
    ctx.out.info(i18n::tr("cli-key-exported"));
    Ok(())
}

fn rotate(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    let key = generate_key();
    store.save(&key)?;
    ctx.out.info(i18n::tr("cli-key-rotated"));
    Ok(())
}
