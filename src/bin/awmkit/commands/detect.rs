use crate::error::{CliError, Result};
use crate::keystore::KeyStore;
use crate::util::{audio_from_context, ensure_file, expand_inputs};
use crate::Context;
use awmkit::Message;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

#[derive(Args)]
pub struct DetectArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,

    /// Input files (supports glob)
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

#[derive(Serialize)]
struct DetectJson {
    file: String,
    status: String,
    tag: Option<String>,
    identity: Option<String>,
    version: Option<u8>,
    timestamp_minutes: Option<u32>,
    timestamp_utc: Option<u64>,
    pattern: Option<String>,
    bit_errors: Option<u32>,
    match_found: Option<bool>,
    error: Option<String>,
}

pub fn run(ctx: &Context, args: &DetectArgs) -> Result<()> {
    let inputs = expand_inputs(&args.inputs)?;
    for input in &inputs {
        ensure_file(input)?;
    }

    let store = KeyStore::new()?;
    let key = store.load()?;
    let audio = audio_from_context(ctx)?;

    if args.json {
        let mut results = Vec::new();
        for input in inputs {
            results.push(detect_one_json(&audio, &key, &input));
        }
        let output = serde_json::to_string_pretty(&results)?;
        println!("{output}");
        return Ok(());
    }

    let progress = if ctx.out.quiet() {
        None
    } else {
        let bar = ProgressBar::new(inputs.len() as u64);
        bar.set_style(
            ProgressStyle::with_template("{prefix} [{bar:40}] {pos}/{len}")
                .map_err(|e| CliError::Message(e.to_string()))?
                .progress_chars("=>-"),
        );
        bar.set_prefix("detect");
        Some(bar)
    };

    let mut ok = 0usize;
    let mut miss = 0usize;
    let mut invalid = 0usize;

    for input in inputs {
        match detect_one(&audio, &key, &input) {
            Ok(DetectOutcome::Found { tag, identity }) => {
                ok += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    if let Some(ref bar) = progress {
                        bar.println(format!(
                            "[OK] {} (tag: {tag}, id: {identity})",
                            input.display()
                        ));
                    } else {
                        ctx.out.info(format!(
                            "[OK] {} (tag: {tag}, id: {identity})",
                            input.display()
                        ));
                    }
                }
            }
            Ok(DetectOutcome::NotFound) => {
                miss += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    if let Some(ref bar) = progress {
                        bar.println(format!("[MISS] {}", input.display()));
                    } else {
                        ctx.out.info(format!("[MISS] {}", input.display()));
                    }
                }
            }
            Ok(DetectOutcome::Invalid(err)) => {
                invalid += 1;
                if let Some(ref bar) = progress {
                    bar.println(format!("[INVALID] {}: {err}", input.display()));
                } else {
                    ctx.out.error(format!("[INVALID] {}: {err}", input.display()));
                }
            }
            Err(err) => {
                invalid += 1;
                if let Some(ref bar) = progress {
                    bar.println(format!("[ERR] {}: {err}", input.display()));
                } else {
                    ctx.out.error(format!("[ERR] {}: {err}", input.display()));
                }
            }
        }

        if let Some(ref bar) = progress {
            bar.inc(1);
        }
    }

    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    if !ctx.out.quiet() {
        ctx.out
            .info(format!("Done: {ok} ok, {miss} missing, {invalid} invalid"));
    }

    if invalid > 0 {
        Err(CliError::Message("one or more files failed".to_string()))
    } else {
        Ok(())
    }
}

enum DetectOutcome {
    Found { tag: String, identity: String },
    NotFound,
    Invalid(String),
}

fn detect_one(
    audio: &awmkit::Audio,
    key: &[u8],
    input: &std::path::Path,
) -> Result<DetectOutcome> {
    match audio.detect(input)? {
        None => Ok(DetectOutcome::NotFound),
        Some(result) => match Message::decode(&result.raw_message, key) {
            Ok(decoded) => Ok(DetectOutcome::Found {
                tag: decoded.tag.to_string(),
                identity: decoded.identity().to_string(),
            }),
            Err(err) => Ok(DetectOutcome::Invalid(err.to_string())),
        },
    }
}

fn detect_one_json(
    audio: &awmkit::Audio,
    key: &[u8],
    input: &std::path::Path,
) -> DetectJson {
    match audio.detect(input) {
        Ok(None) => DetectJson {
            file: input.display().to_string(),
            status: "not_found".to_string(),
            tag: None,
            identity: None,
            version: None,
            timestamp_minutes: None,
            timestamp_utc: None,
            pattern: None,
            bit_errors: None,
            match_found: None,
            error: None,
        },
        Ok(Some(result)) => match Message::decode(&result.raw_message, key) {
            Ok(decoded) => DetectJson {
                file: input.display().to_string(),
                status: "ok".to_string(),
                tag: Some(decoded.tag.to_string()),
                identity: Some(decoded.identity().to_string()),
                version: Some(decoded.version),
                timestamp_minutes: Some(decoded.timestamp_minutes),
                timestamp_utc: Some(decoded.timestamp_utc),
                pattern: Some(result.pattern),
                bit_errors: Some(result.bit_errors),
                match_found: Some(result.match_found),
                error: None,
            },
            Err(err) => DetectJson {
                file: input.display().to_string(),
                status: "invalid_hmac".to_string(),
                tag: None,
                identity: None,
                version: None,
                timestamp_minutes: None,
                timestamp_utc: None,
                pattern: Some(result.pattern),
                bit_errors: Some(result.bit_errors),
                match_found: Some(result.match_found),
                error: Some(err.to_string()),
            },
        },
        Err(err) => DetectJson {
            file: input.display().to_string(),
            status: "error".to_string(),
            tag: None,
            identity: None,
            version: None,
            timestamp_minutes: None,
            timestamp_utc: None,
            pattern: None,
            bit_errors: None,
            match_found: None,
            error: Some(err.to_string()),
        },
    }
}
