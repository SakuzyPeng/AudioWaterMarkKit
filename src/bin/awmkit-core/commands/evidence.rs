use crate::error::{CliError, Result};
use crate::Context;
use awmkit::app::{i18n, AudioEvidence, EvidenceStore};
use clap::{Args, Subcommand};
use fluent_bundle::FluentArgs;
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
        ctx.out.info_user(i18n::tr("cli-evidence-empty"));
        return Ok(());
    }

    for item in &items {
        let sha_prefix = sha_prefix(&item.pcm_sha256);
        let short_path = shorten_middle(&item.file_path, 54);
        let snr_text = if item.snr_status == "ok" {
            format!("{:.2} dB", item.snr_db.unwrap_or_default())
        } else {
            item.snr_status.clone()
        };
        let mut args = FluentArgs::new();
        args.set("id", item.id.to_string());
        args.set("created_at", item.created_at.to_string());
        args.set("identity", item.identity.clone());
        args.set("tag", item.tag.clone());
        args.set("slot", item.key_slot.to_string());
        args.set("snr", snr_text);
        args.set("sha", sha_prefix.to_string());
        args.set("path", short_path);
        ctx.out
            .info_user(i18n::tr_args("cli-evidence-list-row", &args));
    }

    Ok(())
}

/// Internal helper function.
fn show(ctx: &Context, args: &ShowArgs) -> Result<()> {
    let store = EvidenceStore::load()?;
    let Some(item) = store.get_by_id(args.id)? else {
        let mut fmt = FluentArgs::new();
        fmt.set("id", args.id.to_string());
        return Err(CliError::Message(i18n::tr_args(
            "cli-evidence-not-found",
            &fmt,
        )));
    };

    if args.json {
        let text = serde_json::to_string_pretty(&evidence_json(&item))?;
        println!("{text}");
        return Ok(());
    }

    let mut id_args = FluentArgs::new();
    id_args.set("value", item.id.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-id", &id_args));
    let mut created_at_args = FluentArgs::new();
    created_at_args.set("value", item.created_at.to_string());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-created_at",
        &created_at_args,
    ));
    let mut file_path_args = FluentArgs::new();
    file_path_args.set("value", item.file_path.clone());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-file_path",
        &file_path_args,
    ));
    let mut identity_args = FluentArgs::new();
    identity_args.set("value", item.identity.clone());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-identity", &identity_args));
    let mut tag_args = FluentArgs::new();
    tag_args.set("value", item.tag.clone());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-tag", &tag_args));
    let mut version_args = FluentArgs::new();
    version_args.set("value", item.version.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-version", &version_args));
    let mut slot_args = FluentArgs::new();
    slot_args.set("value", item.key_slot.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-key_slot", &slot_args));
    let mut ts_args = FluentArgs::new();
    ts_args.set("value", item.timestamp_minutes.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-timestamp", &ts_args));
    let mut hex_args = FluentArgs::new();
    hex_args.set("value", item.message_hex.clone());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-message_hex", &hex_args));
    let mut sample_rate_args = FluentArgs::new();
    sample_rate_args.set("value", item.sample_rate.to_string());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-sample_rate",
        &sample_rate_args,
    ));
    let mut channels_args = FluentArgs::new();
    channels_args.set("value", item.channels.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-channels", &channels_args));
    let mut sample_count_args = FluentArgs::new();
    sample_count_args.set("value", item.sample_count.to_string());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-sample_count",
        &sample_count_args,
    ));
    let mut sha_args = FluentArgs::new();
    sha_args.set("value", item.pcm_sha256.clone());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-field-pcm_sha256", &sha_args));
    let mut snr_status_args = FluentArgs::new();
    snr_status_args.set("value", item.snr_status.clone());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-snr_status",
        &snr_status_args,
    ));
    if let Some(value) = item.snr_db {
        let mut snr_db_args = FluentArgs::new();
        snr_db_args.set("value", format!("{value:.2}"));
        ctx.out
            .info_user(i18n::tr_args("cli-evidence-field-snr_db", &snr_db_args));
    }
    let mut fp_len_args = FluentArgs::new();
    fp_len_args.set("value", item.chromaprint.len().to_string());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-fingerprint_len",
        &fp_len_args,
    ));
    let mut fp_config_args = FluentArgs::new();
    fp_config_args.set("value", item.fp_config_id.to_string());
    ctx.out.info_user(i18n::tr_args(
        "cli-evidence-field-fp_config_id",
        &fp_config_args,
    ));

    Ok(())
}

/// Internal helper function.
fn remove(ctx: &Context, args: &RemoveArgs) -> Result<()> {
    ensure_yes(args.yes, "remove")?;

    let store = EvidenceStore::load()?;
    if !store.remove_by_id(args.id)? {
        let mut fmt = FluentArgs::new();
        fmt.set("id", args.id.to_string());
        return Err(CliError::Message(i18n::tr_args(
            "cli-evidence-not-found",
            &fmt,
        )));
    }

    let mut fmt = FluentArgs::new();
    fmt.set("id", args.id.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-removed", &fmt));
    Ok(())
}

/// Internal helper function.
fn clear(ctx: &Context, args: &ClearArgs) -> Result<()> {
    ensure_yes(args.yes, "clear")?;
    if args.identity.is_none() && args.tag.is_none() && args.key_slot.is_none() {
        return Err(CliError::Message(i18n::tr("cli-evidence-clear-refuse-all")));
    }

    let store = EvidenceStore::load()?;
    let removed =
        store.clear_filtered(args.identity.as_deref(), args.tag.as_deref(), args.key_slot)?;

    let identity = args.identity.as_deref().unwrap_or("-");
    let tag = args.tag.as_deref().unwrap_or("-");
    let key_slot = args
        .key_slot
        .map_or_else(|| "-".to_string(), |slot| slot.to_string());

    let mut fmt = FluentArgs::new();
    fmt.set("removed", removed.to_string());
    fmt.set("identity", identity.to_string());
    fmt.set("tag", tag.to_string());
    fmt.set("key_slot", key_slot);
    ctx.out
        .info_user(i18n::tr_args("cli-evidence-cleared", &fmt));

    Ok(())
}

/// Internal helper function.
fn ensure_yes(yes: bool, action: &str) -> Result<()> {
    if yes {
        Ok(())
    } else {
        let mut fmt = FluentArgs::new();
        fmt.set("action", action.to_string());
        Err(CliError::Message(i18n::tr_args(
            "cli-evidence-requires-yes",
            &fmt,
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
