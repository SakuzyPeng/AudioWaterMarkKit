use crate::error::{CliError, Result};
use crate::Context;
use awmkit::app::{i18n, AppConfig, AudioEngine};
use awmkit::Tag;
use glob::glob;
use std::path::{Path, PathBuf};

pub fn parse_tag(input: &str) -> Result<Tag> {
    if input.len() == 8 {
        Ok(Tag::parse(input)?)
    } else {
        Ok(Tag::new(input)?)
    }
}

pub fn expand_inputs(values: &[String]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for value in values {
        if is_glob_pattern(value) {
            let mut matched = false;
            let entries = glob(value).map_err(|e| CliError::InvalidGlob(e.to_string()))?;
            for entry in entries {
                let path = entry.map_err(|e| CliError::Glob(e.to_string()))?;
                matched = true;
                out.push(path);
            }
            if !matched {
                return Err(CliError::InputNotFound(value.clone()));
            }
        } else {
            out.push(PathBuf::from(value));
        }
    }

    if out.is_empty() {
        return Err(CliError::Message(i18n::tr("cli-util-no_input_files")));
    }

    Ok(out)
}

pub fn ensure_file(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(CliError::InputNotFound(path.display().to_string()))
    }
}

pub fn audio_from_context(ctx: &Context) -> Result<awmkit::Audio> {
    let config = AppConfig {
        audiowmark_override: ctx.audiowmark.clone(),
    };
    let engine = AudioEngine::new(&config).map_err(|err| match err {
        awmkit::app::AppError::Awmkit(awmkit::Error::AudiowmarkNotFound) => {
            CliError::AudiowmarkNotFound
        }
        other => CliError::from(other),
    })?;
    Ok(engine.audio().clone())
}

pub fn default_output_path(input: &Path) -> Result<PathBuf> {
    awmkit::app::audio_engine::default_output_path(input).map_err(CliError::from)
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}
