use crate::error::{CliError, Result};
use crate::Context;
use awmkit::{Audio, Tag};
use glob::glob;
use std::ffi::OsString;
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
        return Err(CliError::Message("no input files provided".to_string()));
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

pub fn audio_from_context(ctx: &Context) -> Result<Audio> {
    match ctx.audiowmark.as_ref() {
        Some(path) => Audio::with_binary(path).map_err(|_| CliError::AudiowmarkNotFound),
        None => {
            #[cfg(feature = "bundled")]
            {
                if let Ok(path) = crate::bundled::ensure_extracted() {
                    return Audio::with_binary(path).map_err(|_| CliError::AudiowmarkNotFound);
                }
            }
            Audio::new().map_err(|_| CliError::AudiowmarkNotFound)
        }
    }
}

pub fn default_output_path(input: &Path) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .ok_or_else(|| CliError::Message("invalid input file name".to_string()))?;

    let mut name = OsString::from(stem);
    name.push("_wm");

    if let Some(ext) = input.extension() {
        name.push(".");
        name.push(ext);
    }

    let output = input.with_file_name(name);
    if output == input {
        return Err(CliError::Message("output path would overwrite input".to_string()));
    }

    Ok(output)
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}
