use crate::error::{CliError, Result};
use crate::util::{audio_from_context, default_output_path, ensure_file, expand_inputs, parse_tag};
use crate::Context;
use awmkit::app::{build_audio_proof, i18n, EvidenceStore, KeyStore, NewAudioEvidence};
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;
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
        return Err(CliError::Message(i18n::tr("cli-embed-output_single")));
    }

    for input in &inputs {
        ensure_file(input)?;
    }

    let store = KeyStore::new()?;
    let active_slot = store.active_slot()?;
    let key = store.load_slot(active_slot)?;
    let tag = parse_tag(&args.tag)?;
    let message = Message::encode_with_slot(awmkit::CURRENT_VERSION, &tag, &key, active_slot)?;
    let decoded_message = Message::decode(&message, &key)?;
    let evidence_store = match EvidenceStore::load() {
        Ok(store) => Some(store),
        Err(err) => {
            ctx.out.warn(format!("[WARN] evidence: {err}"));
            None
        }
    };

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
                if let Some(evidence_store) = evidence_store.as_ref() {
                    match build_audio_proof(&output) {
                        Ok(proof) => {
                            let insert = NewAudioEvidence {
                                file_path: output.display().to_string(),
                                tag: decoded_message.tag.to_string(),
                                identity: decoded_message.identity().to_string(),
                                version: decoded_message.version,
                                key_slot: decoded_message.key_slot,
                                timestamp_minutes: decoded_message.timestamp_minutes,
                                message_hex: hex::encode(message),
                                sample_rate: proof.sample_rate,
                                channels: proof.channels,
                                sample_count: proof.sample_count,
                                pcm_sha256: proof.pcm_sha256,
                                chromaprint: proof.chromaprint,
                                fp_config_id: proof.fp_config_id,
                            };
                            if let Err(err) = evidence_store.insert(&insert) {
                                ctx.out.warn(format!(
                                    "[WARN] evidence: {} -> {} ({err})",
                                    input.display(),
                                    output.display()
                                ));
                            }
                        }
                        Err(err) => {
                            ctx.out.warn(format!(
                                "[WARN] evidence: {} -> {} ({err})",
                                input.display(),
                                output.display()
                            ));
                        }
                    }
                }
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
        let mut args = FluentArgs::new();
        args.set("success", success.to_string());
        args.set("failed", failed.to_string());
        ctx.out.info(i18n::tr_args("cli-embed-done", &args));
    }

    if failed > 0 {
        Err(CliError::Message(i18n::tr("cli-embed-failed")))
    } else {
        Ok(())
    }
}
