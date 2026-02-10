//! AWMKit CLI

#[cfg(feature = "full-cli")]
fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err.user_message());
        std::process::exit(1);
    }
}

#[cfg(not(feature = "full-cli"))]
fn main() {
    eprintln!("awmkit CLI not enabled. Build with: cargo build --features full-cli --bin awmkit");
    std::process::exit(1);
}

#[cfg(feature = "full-cli")]
mod commands;
#[cfg(feature = "full-cli")]
mod error;
#[cfg(feature = "full-cli")]
mod output;
#[cfg(feature = "full-cli")]
mod util;

#[cfg(feature = "full-cli")]
use awmkit::app::{i18n, AppSettings};
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
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet mode (only errors)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Fallback audiowmark path (used when bundled binary is unavailable)
    #[arg(long, global = true, value_name = "PATH")]
    audiowmark: Option<PathBuf>,

    /// Language (e.g. zh-CN, en-US)
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "full-cli")]
#[derive(Subcommand)]
enum Commands {
    /// Initialize key storage
    Init,

    /// Tag mapping helpers
    Tag {
        #[command(subcommand)]
        command: commands::tag::TagCommand,
    },

    /// Key management
    Key {
        #[command(subcommand)]
        command: KeyCommand,
    },

    /// Encode a watermark message
    Encode(commands::encode::EncodeArgs),

    /// Decode a watermark message
    Decode(commands::decode::DecodeArgs),

    /// Embed watermark into audio files
    Embed(commands::embed::EmbedArgs),

    /// Detect watermark from audio files
    Detect(commands::detect::DetectArgs),

    /// Query and manage evidence records
    Evidence {
        #[command(subcommand)]
        command: commands::evidence::EvidenceCommand,
    },

    /// Show system status
    Status(commands::status::StatusArgs),
}

#[cfg(feature = "full-cli")]
#[derive(Subcommand)]
enum KeyCommand {
    /// Show key info (no key material)
    Show(commands::key::ShowArgs),

    /// Import key from file (binary)
    Import(commands::key::ImportArgs),

    /// Export key to file (binary)
    Export(commands::key::ExportArgs),

    /// Rotate key
    Rotate(commands::key::RotateArgs),

    /// Delete key in one slot
    Delete(commands::key::DeleteArgs),

    /// Slot management
    Slot {
        #[command(subcommand)]
        command: commands::key::SlotCommand,
    },
}

#[cfg(feature = "full-cli")]
struct Context {
    out: Output,
    audiowmark: Option<PathBuf>,
}

#[cfg(feature = "full-cli")]
fn run() -> Result<()> {
    let cli = Cli::parse();

    let settings = AppSettings::load().unwrap_or_default();
    let env_lang = i18n::env_language();
    let lang = cli
        .lang
        .as_deref()
        .or_else(|| env_lang.as_deref())
        .or_else(|| settings.language.as_deref());
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
