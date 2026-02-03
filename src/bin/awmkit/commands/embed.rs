use crate::error::{CliError, Result};
use crate::keystore::KeyStore;
use crate::util::{audio_from_context, default_output_path, ensure_file, expand_inputs, parse_tag};
use crate::Context;
use awmkit::Message;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

#[derive(Args)]
pub struct EmbedArgs {
    /// Tag (1-7 identity or full 8-char tag)
    #[arg(long)]
    pub tag: String,

    /// Watermark strength (1-30)
    #[arg(long, default_value_t = 10)]
    pub strength: u8,

    /// Output file path (single input only)
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Input files (supports glob)
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

pub fn run(ctx: &Context, args: &EmbedArgs) -> Result<()> {
    let mut inputs = expand_inputs(&args.inputs)?;
    if args.output.is_some() && inputs.len() != 1 {
        return Err(CliError::Message(
            "--output only supports a single input file".to_string(),
        ));
    }

    for input in &inputs {
        ensure_file(input)?;
    }

    let store = KeyStore::new()?;
    let key = store.load()?;
    let tag = parse_tag(&args.tag)?;
    let message = Message::encode(awmkit::CURRENT_VERSION, &tag, &key)?;

    let audio = audio_from_context(ctx)?.strength(args.strength);

    let progress = if ctx.out.quiet() {
        None
    } else {
        let bar = ProgressBar::new(inputs.len() as u64);
        bar.set_style(
            ProgressStyle::with_template("{prefix} [{bar:40}] {pos}/{len}")
                .map_err(|e| CliError::Message(e.to_string()))?
                .progress_chars("=>-"),
        );
        bar.set_prefix("embed");
        Some(bar)
    };

    let mut success = 0usize;
    let mut failed = 0usize;

    for input in inputs.drain(..) {
        let output = match &args.output {
            Some(path) => path.clone(),
            None => default_output_path(&input)?,
        };

        let result = audio.embed(&input, &output, &message);
        match result {
            Ok(()) => {
                success += 1;
                if ctx.out.verbose() && !ctx.out.quiet() {
                    if let Some(ref bar) = progress {
                        bar.println(format!("[OK] {} -> {}", input.display(), output.display()));
                    } else {
                        ctx.out
                            .info(format!("[OK] {} -> {}", input.display(), output.display()));
                    }
                }
            }
            Err(err) => {
                failed += 1;
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
        ctx.out.info(format!("Done: {success} succeeded, {failed} failed"));
    }

    if failed > 0 {
        Err(CliError::Message("one or more files failed".to_string()))
    } else {
        Ok(())
    }
}
