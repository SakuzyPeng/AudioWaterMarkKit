use crate::app::error::{AppError, Result};
use fluent_bundle::FluentArgs;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use rust_embed::RustEmbed;
use std::str::FromStr;
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

static FALLBACK_LANG: std::sync::LazyLock<LanguageIdentifier> = std::sync::LazyLock::new(|| {
    LanguageIdentifier::from_str("en-US").unwrap_or_else(|_| LanguageIdentifier::default())
});
static LOADER: std::sync::LazyLock<FluentLanguageLoader> =
    std::sync::LazyLock::new(|| FluentLanguageLoader::new("awmkit", FALLBACK_LANG.clone()));

#[derive(Clone, Copy)]
pub struct LanguageInfo {
    pub id: &'static str,
    pub label: &'static str,
}

static LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        id: "en-US",
        label: "English",
    },
    LanguageInfo {
        id: "zh-CN",
        label: "中文",
    },
];

#[must_use]
pub fn available_languages() -> &'static [LanguageInfo] {
    LANGUAGES
}

pub fn current_language() -> Option<String> {
    LOADER
        .current_languages()
        .first()
        .map(std::string::ToString::to_string)
}

#[must_use]
pub fn env_language() -> Option<String> {
    let raw = std::env::var("LC_ALL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("LANG")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })?;
    let value = raw.split('.').next().unwrap_or(&raw);
    let value = value.split('@').next().unwrap_or(value);
    let normalized = value.replace('_', "-");
    if normalized.is_empty() {
        return None;
    }
    LanguageIdentifier::from_str(&normalized)
        .is_ok()
        .then_some(normalized)
}

pub fn set_language(lang: Option<&str>) -> Result<()> {
    let requested = if let Some(lang) = lang {
        vec![LanguageIdentifier::from_str(lang)
            .map_err(|_| AppError::Message(format!("invalid language identifier: {lang}")))?]
    } else {
        DesktopLanguageRequester::requested_languages()
    };

    let selected = i18n_embed::select(&*LOADER, &Localizations, &requested)
        .map_err(|err| AppError::Message(format!("i18n load failed: {err}")))?;
    if selected.is_empty() {
        LOADER
            .load_fallback_language(&Localizations)
            .map_err(|err| AppError::Message(format!("i18n fallback failed: {err}")))?;
    }
    Ok(())
}

pub fn tr(key: &str) -> String {
    LOADER.get(key)
}

pub fn tr_args(key: &str, args: &FluentArgs) -> String {
    LOADER.get_args_fluent(key, Some(args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_known_key() {
        let set_result = set_language(Some("en-US"));
        assert!(set_result.is_ok());
        let value = tr("ui-tabs-embed");
        assert!(!value.is_empty());
    }

    #[test]
    fn missing_key_falls_back() {
        let set_result = set_language(Some("en-US"));
        assert!(set_result.is_ok());
        let value = tr("missing.key");
        assert!(value.contains("No localization for id"));
    }
}
