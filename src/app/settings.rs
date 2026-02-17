use crate::app::error::{Failure, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub language: Option<String>,
}

impl Preferences {
    /// # Errors
    /// 当配置路径解析或配置文件读取/解析失败时返回错误。.
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        load_from(&path)
    }

    /// # Errors
    /// 当配置路径解析或配置文件写入失败时返回错误。.
    pub fn save_language(lang: &str) -> Result<()> {
        let path = config_path()?;
        let mut settings = load_from(&path).unwrap_or_default();
        settings.language = Some(lang.to_string());
        settings.save_to(&path)
    }

    /// # Errors
    /// 当配置路径解析或文件删除失败时返回错误。.
    pub fn remove_config() -> Result<()> {
        let path = config_path()?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Internal helper method.
    fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = toml::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }
}

/// Internal helper function.
fn load_from(path: &Path) -> Result<Preferences> {
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let raw = fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(Preferences::default());
    }
    let settings = toml::from_str(&raw)?;
    Ok(settings)
}

/// # Errors
/// 当运行环境缺少必要目录环境变量时返回错误。.
pub fn config_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| Failure::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("config.toml");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home =
            std::env::var_os("HOME").ok_or_else(|| Failure::Message("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("config.toml");
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip() {
        let temp = std::env::temp_dir().join("awmkit_settings_test.toml");
        let _ = fs::remove_file(&temp);
        let settings = Preferences {
            language: Some("zh-CN".to_string()),
        };
        let save_result = settings.save_to(&temp);
        assert!(save_result.is_ok());
        let loaded_result = load_from(&temp);
        assert!(loaded_result.is_ok());
        if let Ok(loaded) = loaded_result {
            assert_eq!(loaded.language.as_deref(), Some("zh-CN"));
        }
        let _ = fs::remove_file(&temp);
    }
}
