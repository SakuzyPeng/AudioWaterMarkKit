//! `AWMKit` CLI.

#[cfg(feature = "full-cli")]
fn main() {
    let verbose_requested = cli_arg_has_verbose();
    preload_i18n_for_parser_errors();
    if let Err(err) = run() {
        let rendered = err.render_user_message();
        eprintln!("{}", rendered.user);
        if verbose_requested {
            if let Some(detail) = rendered.detail {
                eprintln!("DETAIL: {detail}");
            }
        }
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
#[command(about = "AWMKit 命令行工具（音频水印嵌入与检测）", version)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "仅 launcher 包装命令支持：\n  cache clean [--db] --yes    清理运行时缓存；加 --db 时同时清理数据库与配置"
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
    /// 初始化签名密钥。
    Init,

    /// 标签映射管理。
    Tag {
        #[command(subcommand)]
        /// Internal field.
        command: commands::tag::Command,
    },

    /// 密钥与槽位管理。
    Key {
        #[command(subcommand)]
        /// Internal field.
        command: KeyCommand,
    },

    /// 编码水印消息（16 字节十六进制）。
    Encode(commands::encode::CmdArgs),

    /// 解码水印消息。
    Decode(commands::decode::CmdArgs),

    /// 向音频文件嵌入水印。
    Embed(commands::embed::CmdArgs),

    /// 从音频文件检测水印。
    Detect(commands::detect::CmdArgs),

    /// 证据记录查询与管理。
    Evidence {
        #[command(subcommand)]
        /// Internal field.
        command: commands::evidence::Command,
    },

    /// 查看系统状态与诊断信息。
    Status(commands::status::CmdArgs),
}

#[cfg(feature = "full-cli")]
#[derive(Subcommand)]
/// Internal enum.
enum KeyCommand {
    /// 查看密钥概览（不显示密钥内容）。
    Show(commands::key::ShowArgs),

    /// 从文件导入密钥（二进制）。
    Import(commands::key::ImportArgs),

    /// 导出密钥到文件（二进制）。
    Export(commands::key::ExportArgs),

    /// 轮换密钥。
    Rotate(commands::key::RotateArgs),

    /// 删除槽位中的密钥。
    Delete(commands::key::DeleteArgs),

    /// 槽位管理。
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

#[cfg(feature = "full-cli")]
fn cli_arg_has_verbose() -> bool {
    std::env::args()
        .skip(1)
        .any(|arg| arg == "-v" || arg == "--verbose")
}

#[cfg(feature = "full-cli")]
fn preload_i18n_for_parser_errors() {
    let env_lang = i18n::env_language();
    let _ = i18n::set_language(env_lang.as_deref());
}
