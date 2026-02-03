use crate::error::{CliError, Result};
use crate::keystore::{generate_key, KeyStore, KEY_LEN};
use crate::Context;
use crate::KeyCommand;
use clap::Args;
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

    ctx.out.info("Key status: configured");
    ctx.out.info(format!("Length: {KEY_LEN} bytes"));
    ctx.out
        .info(format!("Fingerprint (SHA256): {fingerprint}"));
    ctx.out.info(format!("Storage: {}", backend.label()));
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
        ctx.out.info("[OK] key replaced");
    } else {
        ctx.out.info("[OK] key imported");
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
        OpenOptions::new().write(true).create_new(true).open(&args.file)?
    };

    let mut file = file;
    file.write_all(&key)?;
    ctx.out.info("[OK] key exported");
    Ok(())
}

fn rotate(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    let key = generate_key();
    store.save(&key)?;
    ctx.out.info("[OK] key rotated");
    Ok(())
}
