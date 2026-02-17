use crate::app::error::{Failure, Result};
use crate::{Audio, Decoded, DetectResult, Message, Tag, CURRENT_VERSION};
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct Config {
    pub audiowmark_override: Option<PathBuf>,
}

pub struct AudioEngine {
    /// Internal field.
    audio: Audio,
}

#[derive(Debug)]
pub enum DetectOutcome {
    Found { decoded: Decoded, raw: DetectResult },
    NotFound,
    Invalid { raw: DetectResult, error: String },
}

impl AudioEngine {
    /// # Errors
    /// 当初始化底层 `Audio` 失败时返回错误。.
    pub fn new(config: &Config) -> Result<Self> {
        let audio = Audio::new_with_fallback_path(config.audiowmark_override.as_deref())
            .map_err(Failure::from)?;
        Ok(Self { audio })
    }

    /// # Errors
    /// 当消息编码或底层音频嵌入流程失败时返回错误。.
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
            .map_err(Failure::from)
    }

    /// # Errors
    /// 当底层检测流程失败时返回错误。.
    pub fn detect<P: AsRef<Path>>(&self, input: P, key: &[u8]) -> Result<DetectOutcome> {
        match self.audio.detect(input).map_err(Failure::from)? {
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

    #[must_use]
    pub const fn audio(&self) -> &Audio {
        &self.audio
    }
}

/// # Errors
/// 当输入路径无效或输出路径会覆盖输入文件时返回错误。.
pub fn default_output_path(input: &Path) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .ok_or_else(|| Failure::Message("invalid input file name".to_string()))?;

    let mut name = std::ffi::OsString::from(stem);
    name.push("_wm");

    let output_ext = "wav";
    name.push(".");
    name.push(output_ext);

    let output = input.with_file_name(name);
    if output == input {
        return Err(Failure::Message(
            "output path would overwrite input".to_string(),
        ));
    }

    Ok(output)
}
