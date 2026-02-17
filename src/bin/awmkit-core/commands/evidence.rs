use crate::error::{CliError, Result};
use crate::Context;
use awmkit::app::{AudioEvidence, EvidenceStore};
use clap::{Args, Subcommand};
use serde::Serialize;

#[derive(Subcommand)]
/// Internal enum.
pub enum Command {
    /// List evidence records.
    List(ListArgs),

    /// Show one evidence record by id.
    Show(ShowArgs),

    /// Remove one evidence record by id.
    Remove(RemoveArgs),

    /// Clear evidence records by filters.
    Clear(ClearArgs),
}

#[derive(Args)]
/// Internal struct.
pub struct ListArgs {
    /// Filter by identity.
    #[arg(long, value_name = "IDENTITY")]
    pub identity: Option<String>,

    /// Filter by tag.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Filter by key slot.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(0..=31))]
    pub key_slot: Option<u8>,

    /// Max rows to return.
    #[arg(long, value_name = "N", default_value_t = 200, value_parser = clap::value_parser!(u16).range(1..=5000))]
    pub limit: u16,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
/// Internal struct.
pub struct ShowArgs {
    /// Evidence id.
    pub id: i64,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
/// Internal struct.
pub struct RemoveArgs {
    /// Evidence id.
    pub id: i64,

    /// Confirm removal.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args)]
/// Internal struct.
pub struct ClearArgs {
    /// Filter by identity.
    #[arg(long, value_name = "IDENTITY")]
    pub identity: Option<String>,

    /// Filter by tag.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Filter by key slot.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(0..=31))]
    pub key_slot: Option<u8>,

    /// Confirm clear action.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Serialize)]
/// Internal struct.
struct EvidenceJson {
    /// Internal field.
    id: i64,
    /// Internal field.
    created_at: u64,
    /// Internal field.
    file_path: String,
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
    message_hex: String,
    /// Internal field.
    sample_rate: u32,
    /// Internal field.
    channels: u32,
    /// Internal field.
    sample_count: u64,
    /// Internal field.
    pcm_sha256: String,
    /// Internal field.
    snr_db: Option<f64>,
    /// Internal field.
    snr_status: String,
    /// Internal field.
    fingerprint_len: usize,
    /// Internal field.
    fp_config_id: u8,
}

/// Internal helper function.
pub fn run(ctx: &Context, command: Command) -> Result<()> {
    match command {
        Command::List(args) => list(ctx, &args),
        Command::Show(args) => show(ctx, &args),
        Command::Remove(args) => remove(ctx, &args),
        Command::Clear(args) => clear(ctx, &args),
    }
}

/// Internal helper function.
fn list(ctx: &Context, args: &ListArgs) -> Result<()> {
    let store = EvidenceStore::load()?;
    let items = store.list_filtered(
        args.identity.as_deref(),
        args.tag.as_deref(),
        args.key_slot,
        usize::from(args.limit),
    )?;

    if args.json {
        let output: Vec<EvidenceJson> = items.iter().map(evidence_json).collect();
        let text = serde_json::to_string_pretty(&output)?;
        println!("{text}");
        return Ok(());
    }

    if items.is_empty() {
        ctx.out.info("no evidence records");
        return Ok(());
    }

    for item in &items {
        let sha_prefix = sha_prefix(&item.pcm_sha256);
        let short_path = shorten_middle(&item.file_path, 54);
        let snr_text = if item.snr_status == "ok" {
            format!(" snr={:.2}dB", item.snr_db.unwrap_or_default())
        } else {
            format!(" snr={}", item.snr_status)
        };
        ctx.out.info(format!(
            "{} {} {} {} slot={}{} {} {}",
            item.id,
            item.created_at,
            item.identity,
            item.tag,
            item.key_slot,
            snr_text,
            sha_prefix,
            short_path
        ));
    }

    Ok(())
}

/// Internal helper function.
fn show(ctx: &Context, args: &ShowArgs) -> Result<()> {
    let store = EvidenceStore::load()?;
    let Some(item) = store.get_by_id(args.id)? else {
        return Err(CliError::Message(format!(
            "evidence not found: {}",
            args.id
        )));
    };

    if args.json {
        let text = serde_json::to_string_pretty(&evidence_json(&item))?;
        println!("{text}");
        return Ok(());
    }

    ctx.out.info(format!("id={}", item.id));
    ctx.out.info(format!("created_at={}", item.created_at));
    ctx.out.info(format!("file_path={}", item.file_path));
    ctx.out.info(format!("identity={}", item.identity));
    ctx.out.info(format!("tag={}", item.tag));
    ctx.out.info(format!("version={}", item.version));
    ctx.out.info(format!("key_slot={}", item.key_slot));
    ctx.out
        .info(format!("timestamp_minutes={}", item.timestamp_minutes));
    ctx.out.info(format!("message_hex={}", item.message_hex));
    ctx.out.info(format!("sample_rate={}", item.sample_rate));
    ctx.out.info(format!("channels={}", item.channels));
    ctx.out.info(format!("sample_count={}", item.sample_count));
    ctx.out.info(format!("pcm_sha256={}", item.pcm_sha256));
    ctx.out.info(format!("snr_status={}", item.snr_status));
    if let Some(value) = item.snr_db {
        ctx.out.info(format!("snr_db={value:.2}"));
    }
    ctx.out
        .info(format!("fingerprint_len={}", item.chromaprint.len()));
    ctx.out.info(format!("fp_config_id={}", item.fp_config_id));

    Ok(())
}

/// Internal helper function.
fn remove(ctx: &Context, args: &RemoveArgs) -> Result<()> {
    ensure_yes(args.yes, "remove")?;

    let store = EvidenceStore::load()?;
    if !store.remove_by_id(args.id)? {
        return Err(CliError::Message(format!(
            "evidence not found: {}",
            args.id
        )));
    }

    ctx.out.info(format!("removed evidence id={}", args.id));
    Ok(())
}

/// Internal helper function.
fn clear(ctx: &Context, args: &ClearArgs) -> Result<()> {
    ensure_yes(args.yes, "clear")?;
    if args.identity.is_none() && args.tag.is_none() && args.key_slot.is_none() {
        return Err(CliError::Message(
            "refusing to clear all evidence; provide at least one filter".to_string(),
        ));
    }

    let store = EvidenceStore::load()?;
    let removed =
        store.clear_filtered(args.identity.as_deref(), args.tag.as_deref(), args.key_slot)?;

    let identity = args.identity.as_deref().unwrap_or("-");
    let tag = args.tag.as_deref().unwrap_or("-");
    let key_slot = args
        .key_slot
        .map_or_else(|| "-".to_string(), |slot| slot.to_string());

    ctx.out.info(format!(
        "cleared evidence rows={removed} identity={identity} tag={tag} key_slot={key_slot}"
    ));

    Ok(())
}

/// Internal helper function.
fn ensure_yes(yes: bool, action: &str) -> Result<()> {
    if yes {
        Ok(())
    } else {
        Err(CliError::Message(format!(
            "{action} requires --yes confirmation"
        )))
    }
}

/// Internal helper function.
fn evidence_json(item: &AudioEvidence) -> EvidenceJson {
    EvidenceJson {
        id: item.id,
        created_at: item.created_at,
        file_path: item.file_path.clone(),
        tag: item.tag.clone(),
        identity: item.identity.clone(),
        version: item.version,
        key_slot: item.key_slot,
        timestamp_minutes: item.timestamp_minutes,
        message_hex: item.message_hex.clone(),
        sample_rate: item.sample_rate,
        channels: item.channels,
        sample_count: item.sample_count,
        pcm_sha256: item.pcm_sha256.clone(),
        snr_db: item.snr_db,
        snr_status: item.snr_status.clone(),
        fingerprint_len: item.chromaprint.len(),
        fp_config_id: item.fp_config_id,
    }
}

/// Internal helper function.
fn sha_prefix(sha: &str) -> &str {
    let end = sha.char_indices().nth(12).map_or(sha.len(), |(idx, _)| idx);
    &sha[..end]
}

/// Internal helper function.
fn shorten_middle(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let head_len = max_chars / 2;
    let tail_len = max_chars.saturating_sub(head_len + 1);

    let head: String = input.chars().take(head_len).collect();
    let tail: String = input
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("{head}…{tail}")
}
