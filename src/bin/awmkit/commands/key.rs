use crate::error::{CliError, Result};
use crate::Context;
use crate::KeyCommand;
use awmkit::app::{
    generate_key, i18n, AppError, AppSettingsStore, EvidenceStore, KeyStore, KEY_LEN, KEY_SLOT_MAX,
};
use clap::{Args, Subcommand};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Args)]
pub struct ShowArgs {
    /// Operate on a specific slot (0..31). Defaults to active slot.
    #[arg(long, value_name = "N", value_parser = parse_slot_arg)]
    pub slot: Option<u8>,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Key file path (binary)
    pub file: PathBuf,

    /// Target slot (0..31). Defaults to active slot.
    #[arg(long, value_name = "N", value_parser = parse_slot_arg)]
    pub slot: Option<u8>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file path (binary)
    pub file: PathBuf,

    /// Overwrite if exists
    #[arg(long)]
    pub force: bool,

    /// Source slot (0..31). Defaults to active slot.
    #[arg(long, value_name = "N", value_parser = parse_slot_arg)]
    pub slot: Option<u8>,
}

#[derive(Args)]
pub struct RotateArgs {
    /// Target slot (0..31). Defaults to active slot.
    #[arg(long, value_name = "N", value_parser = parse_slot_arg)]
    pub slot: Option<u8>,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Target slot (0..31). Defaults to active slot.
    #[arg(long, value_name = "N", value_parser = parse_slot_arg)]
    pub slot: Option<u8>,

    /// Required confirmation.
    #[arg(long)]
    pub yes: bool,

    /// Also clear evidence rows bound to this slot.
    #[arg(long)]
    pub force: bool,
}

#[derive(Subcommand)]
pub enum SlotCommand {
    /// Show current active slot.
    Current,
    /// Set active slot.
    Use(SlotUseArgs),
    /// List all slots with summary.
    List(SlotListArgs),
    /// Manage slot labels.
    Label {
        #[command(subcommand)]
        command: SlotLabelCommand,
    },
}

#[derive(Subcommand)]
pub enum SlotLabelCommand {
    /// Set label for one slot.
    Set(SlotLabelSetArgs),
    /// Clear label for one slot.
    Clear(SlotLabelClearArgs),
}

#[derive(Args)]
pub struct SlotUseArgs {
    /// Slot id (0..31)
    #[arg(value_parser = parse_slot_arg)]
    pub slot: u8,
}

#[derive(Args)]
pub struct SlotListArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SlotLabelSetArgs {
    /// Slot id (0..31)
    #[arg(value_parser = parse_slot_arg)]
    pub slot: u8,
    /// Human-readable label
    pub label: String,
}

#[derive(Args)]
pub struct SlotLabelClearArgs {
    /// Slot id (0..31)
    #[arg(value_parser = parse_slot_arg)]
    pub slot: u8,
}

#[derive(Serialize)]
struct SlotSummary {
    slot: u8,
    active: bool,
    configured: bool,
    label: Option<String>,
    fingerprint8: Option<String>,
    backend: Option<String>,
    evidence_count: usize,
    last_used_at: Option<u64>,
}

#[derive(Serialize)]
struct ShowJson {
    slot: u8,
    active: bool,
    configured: bool,
    bytes: Option<usize>,
    fingerprint: Option<String>,
    backend: Option<String>,
}

pub fn run(ctx: &Context, command: KeyCommand) -> Result<()> {
    match command {
        KeyCommand::Show(args) => show(ctx, &args),
        KeyCommand::Import(args) => import(ctx, &args),
        KeyCommand::Export(args) => export(ctx, &args),
        KeyCommand::Rotate(args) => rotate(ctx, &args),
        KeyCommand::Delete(args) => delete(ctx, &args),
        KeyCommand::Slot { command } => slot(ctx, command),
    }
}

fn show(ctx: &Context, args: &ShowArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let active_slot = store.active_slot()?;
    let slot = args.slot.unwrap_or(active_slot);
    let loaded = store.load_slot_with_backend(slot);

    if args.json {
        let payload = match loaded {
            Ok((key, backend)) => {
                let fingerprint = key_fingerprint(&key);
                ShowJson {
                    slot,
                    active: slot == active_slot,
                    configured: true,
                    bytes: Some(key.len()),
                    fingerprint: Some(fingerprint),
                    backend: Some(backend.label()),
                }
            }
            Err(_) => ShowJson {
                slot,
                active: slot == active_slot,
                configured: false,
                bytes: None,
                fingerprint: None,
                backend: None,
            },
        };
        let text = serde_json::to_string_pretty(&payload)?;
        ctx.out.info(text);
        return Ok(());
    }

    let mut slot_args = FluentArgs::new();
    slot_args.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-slot", &slot_args));

    match loaded {
        Ok((key, backend)) => {
            let fingerprint = key_fingerprint(&key);
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
        }
        Err(AppError::KeyNotFound) => {
            ctx.out.info(i18n::tr("cli-status-key_not_configured"));
        }
        Err(err) => return Err(err.into()),
    }

    if slot == active_slot {
        ctx.out.info(i18n::tr("cli-key-slot-active"));
    } else {
        let mut args = FluentArgs::new();
        args.set("slot", active_slot.to_string());
        ctx.out
            .info(i18n::tr_args("cli-key-slot-current_active", &args));
    }
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
    let slot = resolve_slot(&store, args.slot)?;
    reject_slot_conflicts(&store, slot, Some(&bytes))?;
    store.save_slot(slot, &bytes)?;

    let mut fmt = FluentArgs::new();
    fmt.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-imported-slot", &fmt));
    Ok(())
}

fn export(ctx: &Context, args: &ExportArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let slot = resolve_slot(&store, args.slot)?;
    let key = store.load_slot(slot)?;

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
    let mut fmt = FluentArgs::new();
    fmt.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-exported-slot", &fmt));
    Ok(())
}

fn rotate(ctx: &Context, args: &RotateArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let slot = resolve_slot(&store, args.slot)?;
    let key = generate_key();
    reject_slot_conflicts(&store, slot, Some(&key))?;
    store.save_slot(slot, &key)?;

    let mut fmt = FluentArgs::new();
    fmt.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-rotated-slot", &fmt));
    Ok(())
}

fn delete(ctx: &Context, args: &DeleteArgs) -> Result<()> {
    if !args.yes {
        return Err(CliError::Message(i18n::tr("cli-key-delete-requires-yes")));
    }
    let store = KeyStore::new()?;
    let slot = resolve_slot(&store, args.slot)?;
    let evidence_store = EvidenceStore::load()?;
    let evidence_count = evidence_store.count_by_slot(slot)?;
    if evidence_count > 0 && !args.force {
        let mut fmt = FluentArgs::new();
        fmt.set("slot", slot.to_string());
        fmt.set("count", evidence_count.to_string());
        return Err(CliError::Message(i18n::tr_args(
            "cli-key-delete-slot-has-evidence",
            &fmt,
        )));
    }

    if args.force && evidence_count > 0 {
        let _ = evidence_store.clear_filtered(None, None, Some(slot))?;
    }
    store.delete_slot(slot)?;

    let mut fmt = FluentArgs::new();
    fmt.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-deleted-slot", &fmt));
    Ok(())
}

fn slot(ctx: &Context, command: SlotCommand) -> Result<()> {
    match command {
        SlotCommand::Current => slot_current(ctx),
        SlotCommand::Use(args) => slot_use(ctx, &args),
        SlotCommand::List(args) => slot_list(ctx, &args),
        SlotCommand::Label { command } => match command {
            SlotLabelCommand::Set(args) => slot_label_set(ctx, &args),
            SlotLabelCommand::Clear(args) => slot_label_clear(ctx, &args),
        },
    }
}

fn slot_current(ctx: &Context) -> Result<()> {
    let store = AppSettingsStore::load()?;
    let slot = store.active_key_slot()?;
    let mut args = FluentArgs::new();
    args.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-slot-current", &args));
    Ok(())
}

fn slot_use(ctx: &Context, args: &SlotUseArgs) -> Result<()> {
    let store = AppSettingsStore::load()?;
    store.set_active_key_slot(args.slot)?;
    let mut fmt = FluentArgs::new();
    fmt.set("slot", args.slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-slot-set", &fmt));
    Ok(())
}

fn slot_list(ctx: &Context, args: &SlotListArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let settings = AppSettingsStore::load()?;
    let evidence_store = EvidenceStore::load()?;
    let active = settings.active_key_slot()?;

    let mut summaries = Vec::new();
    for slot in 0..=KEY_SLOT_MAX {
        let (configured, backend, fingerprint8) = match store.load_slot_with_backend(slot) {
            Ok((key, backend)) => {
                let fp = key_fingerprint(&key);
                (
                    true,
                    Some(backend.label()),
                    Some(fp.chars().take(8).collect()),
                )
            }
            Err(_) => (false, None, None),
        };
        let label = settings.slot_label(slot)?;
        let usage = evidence_store.usage_by_slot(slot)?;
        summaries.push(SlotSummary {
            slot,
            active: slot == active,
            configured,
            label,
            fingerprint8,
            backend,
            evidence_count: usage.count,
            last_used_at: usage.last_created_at,
        });
    }

    if args.json {
        ctx.out.info(serde_json::to_string_pretty(&summaries)?);
        return Ok(());
    }

    for item in summaries {
        let marker = if item.active { "*" } else { " " };
        let configured = if item.configured {
            "configured"
        } else {
            "empty"
        };
        let label = item.label.unwrap_or_else(|| "-".to_string());
        let fp = item.fingerprint8.unwrap_or_else(|| "-".to_string());
        let backend = item.backend.unwrap_or_else(|| "-".to_string());
        let last = item
            .last_used_at
            .map_or_else(|| "-".to_string(), |value| value.to_string());
        ctx.out.info(format!(
            "[{slot:02}]{marker} {configured} label={label} fp={fp} backend={backend} evidence={evidence} last={last}",
            slot = item.slot,
            evidence = item.evidence_count,
        ));
    }

    Ok(())
}

fn slot_label_set(ctx: &Context, args: &SlotLabelSetArgs) -> Result<()> {
    let store = AppSettingsStore::load()?;
    store.set_slot_label(args.slot, &args.label)?;
    let mut fmt = FluentArgs::new();
    fmt.set("slot", args.slot.to_string());
    fmt.set("label", args.label.as_str());
    ctx.out.info(i18n::tr_args("cli-key-slot-label-set", &fmt));
    Ok(())
}

fn slot_label_clear(ctx: &Context, args: &SlotLabelClearArgs) -> Result<()> {
    let store = AppSettingsStore::load()?;
    store.clear_slot_label(args.slot)?;
    let mut fmt = FluentArgs::new();
    fmt.set("slot", args.slot.to_string());
    ctx.out
        .info(i18n::tr_args("cli-key-slot-label-cleared", &fmt));
    Ok(())
}

pub fn generate_for_active_slot() -> Result<u8> {
    let store = KeyStore::new()?;
    let slot = store.active_slot()?;
    reject_slot_conflicts(&store, slot, None)?;
    let key = generate_key();
    store.save_slot(slot, &key)?;
    Ok(slot)
}

fn resolve_slot(store: &KeyStore, slot: Option<u8>) -> Result<u8> {
    match slot {
        Some(value) => Ok(value),
        None => store.active_slot().map_err(CliError::from),
    }
}

fn reject_slot_conflicts(store: &KeyStore, slot: u8, candidate_key: Option<&[u8]>) -> Result<()> {
    if store.exists_slot(slot) {
        let mut args = FluentArgs::new();
        args.set("slot", slot.to_string());
        return Err(CliError::Message(i18n::tr_args(
            "cli-key-conflict-slot-occupied",
            &args,
        )));
    }

    let evidence_store = EvidenceStore::load()?;
    let usage = evidence_store.count_by_slot(slot)?;
    if usage > 0 {
        let mut args = FluentArgs::new();
        args.set("slot", slot.to_string());
        args.set("count", usage.to_string());
        return Err(CliError::Message(i18n::tr_args(
            "cli-key-conflict-slot-has-evidence",
            &args,
        )));
    }

    if let Some(candidate_key) = candidate_key {
        let target_fp = key_fingerprint(candidate_key);
        let mut conflicts = Vec::new();
        for other_slot in store.list_configured_slots() {
            if other_slot == slot {
                continue;
            }
            if let Ok(existing_key) = store.load_slot(other_slot) {
                let fp = key_fingerprint(&existing_key);
                if fp == target_fp {
                    conflicts.push(other_slot.to_string());
                }
            }
        }
        if !conflicts.is_empty() {
            let mut args = FluentArgs::new();
            args.set("slot", slot.to_string());
            args.set("conflicts", conflicts.join(","));
            return Err(CliError::Message(i18n::tr_args(
                "cli-key-conflict-duplicate-fingerprint",
                &args,
            )));
        }
    }

    Ok(())
}

fn key_fingerprint(key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hex::encode(hasher.finalize())
}

fn parse_slot_arg(raw: &str) -> std::result::Result<u8, String> {
    let slot = raw
        .parse::<u8>()
        .map_err(|_| format!("invalid slot: {raw}"))?;
    if slot <= KEY_SLOT_MAX {
        Ok(slot)
    } else {
        Err(format!(
            "invalid slot: {slot} (expected 0..={KEY_SLOT_MAX})"
        ))
    }
}
