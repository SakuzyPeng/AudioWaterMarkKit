//! `AWMKit` CLI.

#[cfg(feature = "full-cli")]
fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err.user_message());
        std::process::exit(1);
    }
}

#[cfg(not(feature = "full-cli"))]
fn main() {
    eprintln!(
        "awmkit core CLI not enabled. Build with: cargo build --features full-cli --bin awmkit-core"
    );
    std::process::exit(1);
}

#[cfg(feature = "full-cli")]
/// Internal module.
mod commands;
#[cfg(feature = "full-cli")]
/// Internal module.
mod error;
#[cfg(feature = "full-cli")]
/// Internal module.
mod output;
#[cfg(feature = "full-cli")]
/// Internal module.
mod util;

#[cfg(feature = "full-cli")]
use awmkit::app::{i18n, Preferences};
#[cfg(feature = "full-cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "full-cli")]
use error::{CliError, Result};
#[cfg(feature = "full-cli")]
use output::Output;
#[cfg(feature = "full-cli")]
use std::path::PathBuf;

#[cfg(feature = "full-cli")]
#[derive(Parser)]
#[command(name = "awmkit")]
#[command(about = "Audio Watermark Kit CLI", version)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Launcher-only command (via `awmkit` wrapper):\n  cache clean [--db] --yes    Clean extracted runtime; add --db to also remove database and config"
)]
/// Internal struct.
struct Cli {
    /// Verbose output.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet mode (only errors).
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Fallback audiowmark path (used when bundled binary is unavailable).
    #[arg(long, global = true, value_name = "PATH")]
    audiowmark: Option<PathBuf>,

    /// Language (e.g. zh-CN, en-US).
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,

    #[command(subcommand)]
    /// Internal field.
    command: Commands,
}

#[cfg(feature = "full-cli")]
#[derive(Subcommand)]
/// Internal enum.
enum Commands {
    /// Initialize key storage.
    Init,

    /// Tag mapping helpers.
    Tag {
        #[command(subcommand)]
        /// Internal field.
        command: commands::tag::Command,
    },

    /// Key management.
    Key {
        #[command(subcommand)]
        /// Internal field.
        command: KeyCommand,
    },

    /// Encode a watermark message.
    Encode(commands::encode::CmdArgs),

    /// Decode a watermark message.
    Decode(commands::decode::CmdArgs),

    /// Embed watermark into audio files.
    Embed(commands::embed::CmdArgs),

    /// Detect watermark from audio files.
    Detect(commands::detect::CmdArgs),

    /// Query and manage evidence records.
    Evidence {
        #[command(subcommand)]
        /// Internal field.
        command: commands::evidence::Command,
    },

    /// Show system status.
    Status(commands::status::CmdArgs),
}

#[cfg(feature = "full-cli")]
#[derive(Subcommand)]
/// Internal enum.
enum KeyCommand {
    /// Show key info (no key material).
    Show(commands::key::ShowArgs),

    /// Import key from file (binary).
    Import(commands::key::ImportArgs),

    /// Export key to file (binary).
    Export(commands::key::ExportArgs),

    /// Rotate key.
    Rotate(commands::key::RotateArgs),

    /// Delete key in one slot.
    Delete(commands::key::DeleteArgs),

    /// Slot management.
    Slot {
        #[command(subcommand)]
        /// Internal field.
        command: commands::key::SlotCommand,
    },
}

#[cfg(feature = "full-cli")]
/// Internal struct.
struct Context {
    /// Internal field.
    out: Output,
    /// Internal field.
    audiowmark: Option<PathBuf>,
}

#[cfg(feature = "full-cli")]
/// Internal helper function.
fn run() -> Result<()> {
    let cli = Cli::parse();

    let settings = Preferences::load().unwrap_or_default();
    let env_lang = i18n::env_language();
    let lang = cli
        .lang
        .as_deref()
        .or(env_lang.as_deref())
        .or(settings.language.as_deref());
    i18n::set_language(lang).map_err(CliError::from)?;

    if cli.quiet && cli.verbose {
        return Err(CliError::Message(i18n::tr(
            "cli-error-quiet_verbose_conflict",
        )));
    }

    let ctx = Context {
        out: Output::new(cli.quiet, cli.verbose),
        audiowmark: cli.audiowmark,
    };

    match cli.command {
        Commands::Init => commands::init::run(&ctx),
        Commands::Tag { command } => commands::tag::run(&ctx, command),
        Commands::Key { command } => commands::key::run(&ctx, command),
        Commands::Encode(args) => commands::encode::run(&ctx, &args),
        Commands::Decode(args) => commands::decode::run(&ctx, &args),
        Commands::Embed(args) => commands::embed::run(&ctx, &args),
        Commands::Detect(args) => commands::detect::run(&ctx, &args),
        Commands::Evidence { command } => commands::evidence::run(&ctx, command),
        Commands::Status(args) => commands::status::run(&ctx, &args),
    }
}
