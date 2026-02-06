use crate::app::error::{AppError, Result};
use crate::{Audio, DetectResult, Message, MessageResult, Tag, CURRENT_VERSION};
use std::path::{Path, PathBuf};

#[cfg(feature = "bundled")]
use crate::app::bundled;

#[derive(Default, Clone)]
pub struct AppConfig {
    pub audiowmark_override: Option<PathBuf>,
}

pub struct AudioEngine {
    audio: Audio,
}

#[derive(Debug)]
pub enum DetectOutcome {
    Found {
        decoded: MessageResult,
        raw: DetectResult,
    },
    NotFound,
    Invalid {
        raw: DetectResult,
        error: String,
    },
}

impl AudioEngine {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let audio = match config.audiowmark_override.as_ref() {
            Some(path) => Audio::with_binary(path).map_err(AppError::from)?,
            None => {
                #[cfg(feature = "bundled")]
                {
                    if let Ok(path) = bundled::ensure_extracted() {
                        return Ok(Self {
                            audio: Audio::with_binary(path).map_err(AppError::from)?,
                        });
                    }
                }
                Audio::new().map_err(AppError::from)?
            }
        };
        Ok(Self { audio })
    }

    pub fn embed<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        tag: &Tag,
        key: &[u8],
        strength: u8,
    ) -> Result<()> {
        let audio = self.audio.clone().strength(strength);
        audio
            .embed_with_tag(input, output, CURRENT_VERSION, tag, key)
            .map(|_| ())
            .map_err(AppError::from)
    }

    pub fn detect<P: AsRef<Path>>(&self, input: P, key: &[u8]) -> Result<DetectOutcome> {
        match self.audio.detect(input).map_err(AppError::from)? {
            None => Ok(DetectOutcome::NotFound),
            Some(raw) => match Message::decode(&raw.raw_message, key) {
                Ok(decoded) => Ok(DetectOutcome::Found { decoded, raw }),
                Err(err) => Ok(DetectOutcome::Invalid {
                    raw,
                    error: err.to_string(),
                }),
            },
        }
    }

    pub fn audio(&self) -> &Audio {
        &self.audio
    }
}

pub fn default_output_path(input: &Path) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .ok_or_else(|| AppError::Message("invalid input file name".to_string()))?;

    let mut name = std::ffi::OsString::from(stem);
    name.push("_wm");

    if let Some(ext) = input.extension() {
        name.push(".");
        name.push(ext);
    }

    let output = input.with_file_name(name);
    if output == input {
        return Err(AppError::Message(
            "output path would overwrite input".to_string(),
        ));
    }

    Ok(output)
}
