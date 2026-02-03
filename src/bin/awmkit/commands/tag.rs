use crate::error::{CliError, Result};
use crate::util::parse_tag;
use crate::Context;
use awmkit::charset::CHARSET;
use awmkit::Tag;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Subcommand)]
pub enum TagCommand {
    /// Suggest a tag from a username (deterministic, no storage)
    Suggest(SuggestArgs),

    /// Save a username -> tag mapping
    Save(SaveArgs),

    /// List saved mappings
    List(ListArgs),

    /// Remove a saved mapping
    Remove(RemoveArgs),

    /// Clear all mappings
    Clear,
}

#[derive(Args)]
pub struct SuggestArgs {
    /// Username to map
    pub username: String,
}

#[derive(Args)]
pub struct SaveArgs {
    /// Username to map
    pub username: String,

    /// Use a specific tag (default: deterministic suggestion)
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Overwrite existing mapping
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Username to remove
    pub username: String,
}

#[derive(Default, Serialize, Deserialize)]
struct TagStore {
    version: u8,
    entries: Vec<TagEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct TagEntry {
    username: String,
    tag: String,
    created_at: u64,
}

pub fn run(ctx: &Context, command: TagCommand) -> Result<()> {
    match command {
        TagCommand::Suggest(args) => suggest(ctx, &args),
        TagCommand::Save(args) => save(ctx, &args),
        TagCommand::List(args) => list(ctx, &args),
        TagCommand::Remove(args) => remove(ctx, &args),
        TagCommand::Clear => clear(ctx),
    }
}

fn suggest(ctx: &Context, args: &SuggestArgs) -> Result<()> {
    let username = normalize_username(&args.username)?;
    let tag = suggest_tag(&username)?;
    ctx.out.info(tag.as_str());
    Ok(())
}

fn save(ctx: &Context, args: &SaveArgs) -> Result<()> {
    let username = normalize_username(&args.username)?;
    let tag = match args.tag.as_ref() {
        Some(tag) => parse_tag(tag)?.as_str().to_string(),
        None => suggest_tag(&username)?.as_str().to_string(),
    };

    let path = tags_path()?;
    let mut store = load_store(&path)?;

    if let Some(existing) = store
        .entries
        .iter_mut()
        .find(|entry| entry.username == username)
    {
        if existing.tag == tag {
            ctx.out.info(format!("already saved: {} -> {}", username, tag));
            return Ok(());
        }
        if !args.force {
            return Err(CliError::Message(format!(
                "mapping exists for {username}; use --force to overwrite"
            )));
        }
        existing.tag = tag.clone();
        existing.created_at = now_ts()?;
    } else {
        store.entries.push(TagEntry {
            username: username.clone(),
            tag: tag.clone(),
            created_at: now_ts()?,
        });
    }

    store.version = 1;
    store.entries.sort_by(|a, b| a.username.cmp(&b.username));
    save_store(&path, &store)?;
    ctx.out.info(format!("saved: {} -> {}", username, tag));
    Ok(())
}

fn list(ctx: &Context, args: &ListArgs) -> Result<()> {
    let path = tags_path()?;
    let store = load_store(&path)?;

    if args.json {
        let output = serde_json::to_string_pretty(&store)?;
        println!("{output}");
        return Ok(());
    }

    if store.entries.is_empty() {
        ctx.out.info("no saved tags");
        return Ok(());
    }

    for entry in store.entries {
        ctx.out.info(format!("{} -> {}", entry.username, entry.tag));
    }

    Ok(())
}

fn remove(ctx: &Context, args: &RemoveArgs) -> Result<()> {
    let username = normalize_username(&args.username)?;
    let path = tags_path()?;
    let mut store = load_store(&path)?;
    let before = store.entries.len();
    store.entries.retain(|entry| entry.username != username);
    if store.entries.len() == before {
        return Err(CliError::Message(format!("mapping not found: {username}")));
    }
    save_store(&path, &store)?;
    ctx.out.info(format!("removed: {username}"));
    Ok(())
}

fn clear(ctx: &Context) -> Result<()> {
    let path = tags_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    ctx.out.info("cleared all mappings");
    Ok(())
}

fn normalize_username(username: &str) -> Result<String> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(CliError::Message("username cannot be empty".to_string()));
    }
    Ok(trimmed.to_string())
}

fn suggest_tag(username: &str) -> Result<Tag> {
    let mut hasher = Sha256::new();
    hasher.update(username.as_bytes());
    let hash = hasher.finalize();
    let identity = hash_to_identity(&hash);
    Tag::new(&identity).map_err(CliError::from)
}

fn hash_to_identity(hash: &[u8]) -> String {
    let mut out = String::with_capacity(7);
    let mut acc: u64 = 0;
    let mut acc_bits: u8 = 0;

    for &b in hash {
        acc = (acc << 8) | u64::from(b);
        acc_bits += 8;
        while acc_bits >= 5 && out.len() < 7 {
            let shift = acc_bits - 5;
            let idx = ((acc >> shift) & 0x1F) as usize;
            out.push(CHARSET[idx] as char);
            acc_bits -= 5;
        }
        if out.len() >= 7 {
            break;
        }
    }

    out
}

fn tags_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| CliError::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("tags.json");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| CliError::Message("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("tags.json");
        Ok(path)
    }
}

fn load_store(path: &Path) -> Result<TagStore> {
    if !path.exists() {
        return Ok(TagStore::default());
    }
    let raw = fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(TagStore::default());
    }
    let store: TagStore = serde_json::from_str(&raw)?;
    Ok(store)
}

fn save_store(path: &Path, store: &TagStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(store)?;
    fs::write(path, data)?;
    Ok(())
}

fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CliError::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}
