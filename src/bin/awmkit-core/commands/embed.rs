use crate::error::{CliError, Result};
use crate::util::{
    audio_from_context, default_output_path, ensure_file, expand_inputs, parse_tag, CliLayout,
};
use crate::Context;
use awmkit::app::{
    analyze_snr, build_audio_proof, i18n, key_id_from_key_material, EvidenceStore, KeyStore,
    NewAudioEvidence, TagStore, SNR_STATUS_OK,
};
use awmkit::{Error as AwmError, Message};
use clap::Args;
use fluent_bundle::FluentArgs;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

/// Internal constant.
const EMBED_PROGRESS_TEMPLATE: &str = "{prefix} [{bar:40}] {pos}/{len}";

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// Tag (1-7 identity or full 8-char tag).
    #[arg(long)]
    pub tag: String,

    /// Watermark strength (1-30).
    #[arg(long, default_value_t = 10)]
    pub strength: u8,

    /// Channel layout (default: auto).
    #[arg(long, value_enum, default_value_t = CliLayout::Auto)]
    pub layout: CliLayout,

    /// Output file path (single input only).
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Input files (supports glob).
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<String>,
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    let inputs = expand_inputs(&args.inputs)?;
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
    let layout = args.layout.to_channel_layout();

    let progress = if ctx.out.quiet() {
        None
    } else {
        let bar = ProgressBar::new(inputs.len() as u64);
        bar.set_style(
            ProgressStyle::with_template(EMBED_PROGRESS_TEMPLATE)
                .map_err(|e| CliError::Message(e.to_string()))?
                .progress_chars("=>-"),
        );
        bar.set_prefix("embed");
        Some(bar)
    };

    let mut success = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut failure_details: Vec<String> = Vec::new();

    if !ctx.out.quiet() {
        ctx.out
            .info("[INFO] multichannel smart routing enabled (default: LFE skip)");
        if ctx.out.verbose() {
            let parallelism = std::thread::available_parallelism()
                .map(std::num::NonZero::get)
                .unwrap_or(1);
            ctx.out.info(format!(
                "[INFO] multichannel route steps use Rayon parallel execution (max workers: {parallelism})"
            ));
        }
    }

    for input in inputs {
        let output = match &args.output {
            Some(path) => path.clone(),
            None => default_output_path(&input)?,
        };

        match audio.detect_multichannel(&input, layout) {
            Ok(detect) => {
                if detect.best.is_some() {
                    skipped += 1;
                    if let Some(ref bar) = progress {
                        bar.println(format!("[SKIP] {}: already watermarked", input.display()));
                    } else if !ctx.out.quiet() {
                        ctx.out
                            .warn(format!("[SKIP] {}: already watermarked", input.display()));
                    }
                    if let Some(ref bar) = progress {
                        bar.inc(1);
                    }
                    continue;
                }
            }
            Err(err) => {
                if matches!(err, AwmError::AdmUnsupported(_)) {
                    if let Some(ref bar) = progress {
                        bar.println(format!(
                            "[WARN] {}: ADM detect 暂不支持，跳过预检并继续嵌入",
                            input.display()
                        ));
                    } else if !ctx.out.quiet() {
                        ctx.out.warn(format!(
                            "[WARN] {}: ADM detect 暂不支持，跳过预检并继续嵌入",
                            input.display()
                        ));
                    }
                } else {
                    failed += 1;
                    if let Some(ref bar) = progress {
                        bar.println(format!("[ERR] {}: precheck failed: {err}", input.display()));
                        bar.inc(1);
                    } else {
                        crate::output::Output::error(format!(
                            "[ERR] {}: precheck failed: {err}",
                            input.display()
                        ));
                    }
                    continue;
                }
            }
        }

        let result = audio.embed_multichannel(&input, &output, &message, layout);
        match result {
            Ok(()) => {
                success += 1;
                let snr = analyze_snr(&input, &output);
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
                                key_id: key_id_from_key_material(&key),
                                is_forced_embed: false,
                                snr_db: snr.snr_db,
                                snr_status: snr.status.clone(),
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
                if !ctx.out.quiet() {
                    let snr_text = if snr.status == SNR_STATUS_OK {
                        format!("SNR {:.2} dB", snr.snr_db.unwrap_or_default())
                    } else {
                        let reason = snr.detail.unwrap_or_else(|| snr.status.clone());
                        format!("SNR unavailable ({reason})")
                    };
                    let line = format!(
                        "[OK] {} -> {} | {}",
                        input.display(),
                        output.display(),
                        snr_text
                    );
                    ctx.out.info(line);
                }
            }
            Err(err) => {
                failed += 1;
                failure_details.push(format!("{}: {err}", input.display()));
                if let Some(ref bar) = progress {
                    bar.println(format!("[ERR] {}: {err}", input.display()));
                } else {
                    crate::output::Output::error(format!("[ERR] {}: {err}", input.display()));
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
        if skipped > 0 {
            ctx.out.warn(format!("已跳过 {skipped} 个已含水印文件"));
        }
        if !failure_details.is_empty() {
            ctx.out.warn("失败详情：");
            for detail in failure_details.iter().take(8) {
                ctx.out.warn(format!("- {detail}"));
            }
            let remain = failure_details.len().saturating_sub(8);
            if remain > 0 {
                ctx.out.warn(format!("- 其余 {remain} 条失败详情已省略"));
            }
        }
    }

    if success > 0 {
        match TagStore::load() {
            Ok(mut store) => match store.save_if_absent(decoded_message.identity(), &tag) {
                Ok(inserted) if inserted && !ctx.out.quiet() => {
                    ctx.out.info(format!(
                        "已自动保存映射：{} -> {}",
                        decoded_message.identity(),
                        decoded_message.tag
                    ));
                }
                Ok(_) => {}
                Err(err) => {
                    ctx.out
                        .warn(format!("[WARN] tag mapping: save failed ({err})"));
                }
            },
            Err(err) => {
                ctx.out
                    .warn(format!("[WARN] tag mapping: load failed ({err})"));
            }
        }
    }

    if failed > 0 {
        Err(CliError::Message(i18n::tr("cli-embed-failed")))
    } else {
        Ok(())
    }
}
