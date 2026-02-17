use crate::error::Result;
use crate::util::parse_tag;
use crate::Context;
use awmkit::app::{i18n, TagEntry, TagStore};
use clap::{Args, Subcommand};
use fluent_bundle::FluentArgs;
use serde::Serialize;

#[derive(Subcommand)]
/// Internal enum.
pub enum Command {
    /// Suggest a tag from a username (deterministic, no storage).
    Suggest(SuggestArgs),

    /// Save a username -> tag mapping.
    Save(SaveArgs),

    /// List saved mappings.
    List(ListArgs),

    /// Remove a saved mapping.
    Remove(RemoveArgs),

    /// Clear all mappings.
    Clear,
}

#[derive(Args)]
/// Internal struct.
pub struct SuggestArgs {
    /// Username to map.
    pub username: String,
}

#[derive(Args)]
/// Internal struct.
pub struct SaveArgs {
    /// Username to map.
    pub username: String,

    /// Use a specific tag (default: deterministic suggestion).
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Overwrite existing mapping.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
/// Internal struct.
pub struct ListArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
/// Internal struct.
pub struct RemoveArgs {
    /// Username to remove.
    pub username: String,
}

#[derive(Serialize)]
/// Internal struct.
struct TagStoreOutput {
    /// Internal field.
    version: u8,
    /// Internal field.
    entries: Vec<TagEntry>,
}

/// Internal helper function.
pub fn run(ctx: &Context, command: Command) -> Result<()> {
    match command {
        Command::Suggest(args) => suggest(ctx, &args),
        Command::Save(args) => save(ctx, &args),
        Command::List(args) => list(ctx, &args),
        Command::Remove(args) => remove(ctx, &args),
        Command::Clear => clear(ctx),
    }
}

/// Internal helper function.
fn suggest(ctx: &Context, args: &SuggestArgs) -> Result<()> {
    let tag = TagStore::suggest(&args.username)?;
    ctx.out.info(tag.as_str());
    Ok(())
}

/// Internal helper function.
fn save(ctx: &Context, args: &SaveArgs) -> Result<()> {
    let tag = match args.tag.as_ref() {
        Some(tag) => parse_tag(tag)?,
        None => TagStore::suggest(&args.username)?,
    };

    let mut store = TagStore::load()?;
    store.save(&args.username, &tag, args.force)?;
    let mut args_i18n = FluentArgs::new();
    args_i18n.set("username", args.username.as_str());
    args_i18n.set("tag", tag.as_str());
    ctx.out.info(i18n::tr_args("cli-tag-saved", &args_i18n));
    Ok(())
}

/// Internal helper function.
fn list(ctx: &Context, args: &ListArgs) -> Result<()> {
    let store = TagStore::load()?;

    if args.json {
        let output = TagStoreOutput {
            version: 1,
            entries: store.list().to_vec(),
        };
        let output = serde_json::to_string_pretty(&output)?;
        println!("{output}");
        return Ok(());
    }

    if store.list().is_empty() {
        ctx.out.info(i18n::tr("cli-tag-none"));
        return Ok(());
    }

    for entry in store.list() {
        ctx.out.info(format!("{} -> {}", entry.username, entry.tag));
    }

    Ok(())
}

/// Internal helper function.
fn remove(ctx: &Context, args: &RemoveArgs) -> Result<()> {
    let mut store = TagStore::load()?;
    store.remove(&args.username)?;
    let mut args_i18n = FluentArgs::new();
    args_i18n.set("username", args.username.as_str());
    ctx.out.info(i18n::tr_args("cli-tag-removed", &args_i18n));
    Ok(())
}

/// Internal helper function.
fn clear(ctx: &Context) -> Result<()> {
    let mut store = TagStore::load()?;
    store.clear()?;
    ctx.out.info(i18n::tr("cli-tag-cleared"));
    Ok(())
}
