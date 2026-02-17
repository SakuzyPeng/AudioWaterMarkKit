use crate::app::error::{AppError, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimum valid key slot.
pub const KEY_SLOT_MIN: u8 = 0;
/// Maximum valid key slot.
pub const KEY_SLOT_MAX: u8 = 31;
/// Internal constant.
const ACTIVE_KEY_SLOT_KEY: &str = "active_key_slot";
/// Internal constant.
const UI_LANGUAGE_KEY: &str = "ui_language";
/// Internal constant.
const UI_LANG_ZH_CN: &str = "zh-CN";
/// Internal constant.
const UI_LANG_EN_US: &str = "en-US";

/// App-level settings store backed by sqlite.
pub struct AppSettingsStore {
    /// Internal field.
    conn: Connection,
    /// Internal field.
    path: PathBuf,
}

impl AppSettingsStore {
    /// Open settings store at shared awmkit sqlite database.
    ///
    /// # Errors
    /// 当数据库路径解析、目录创建或 `SQLite` 打开失败时返回错误。
    pub fn load() -> Result<Self> {
        let path = db_path()?;
        let conn = open_db(&path)?;
        Ok(Self { conn, path })
    }

    /// Read active key slot. Missing/invalid value falls back to 0.
    ///
    /// # Errors
    /// 当读取 `SQLite` 配置失败时返回错误。
    pub fn active_key_slot(&self) -> Result<u8> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1 LIMIT 1")?;
        let value: Option<String> = stmt
            .query_row(params![ACTIVE_KEY_SLOT_KEY], |row| row.get(0))
            .optional()?;

        let Some(raw) = value else {
            return Ok(KEY_SLOT_MIN);
        };

        match raw.parse::<u8>() {
            Ok(slot) if is_valid_slot(slot) => Ok(slot),
            _ => Ok(KEY_SLOT_MIN),
        }
    }

    /// Persist active key slot.
    ///
    /// # Errors
    /// 当槽位超出范围或 `SQLite` 写入失败时返回错误。
    pub fn set_active_key_slot(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        self.conn.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![ACTIVE_KEY_SLOT_KEY, slot.to_string(), now_ts()?],
        )?;
        Ok(())
    }

    /// Read UI language override. Missing/invalid value returns None.
    ///
    /// # Errors
    /// 当读取 `SQLite` 配置失败时返回错误。
    pub fn ui_language(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1 LIMIT 1")?;
        let value: Option<String> = stmt
            .query_row(params![UI_LANGUAGE_KEY], |row| row.get(0))
            .optional()?;

        let Some(raw) = value else {
            return Ok(None);
        };

        Ok(normalize_ui_language(&raw).map(std::string::ToString::to_string))
    }

    /// Persist UI language override.
    /// - `Some("zh-CN" | "en-US")`: set value
    /// - `None`: clear value (use system default on app side)
    ///
    /// # Errors
    /// 当语言值不受支持或 `SQLite` 写入失败时返回错误。
    pub fn set_ui_language(&self, lang: Option<&str>) -> Result<()> {
        let now = now_ts()?;
        match lang {
            Some(raw) => {
                let Some(normalized) = normalize_ui_language(raw) else {
                    return Err(AppError::Message(format!(
                        "invalid ui language: {raw} (expected {UI_LANG_ZH_CN} or {UI_LANG_EN_US})"
                    )));
                };

                self.conn.execute(
                    "INSERT INTO app_settings (key, value, updated_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value,
                        updated_at = excluded.updated_at",
                    params![UI_LANGUAGE_KEY, normalized, now],
                )?;
            }
            None => {
                self.conn.execute(
                    "DELETE FROM app_settings WHERE key = ?1",
                    params![UI_LANGUAGE_KEY],
                )?;
            }
        }
        Ok(())
    }

    /// Get human-readable label for a slot.
    ///
    /// # Errors
    /// 当槽位超出范围或 `SQLite` 查询失败时返回错误。
    pub fn slot_label(&self, slot: u8) -> Result<Option<String>> {
        validate_slot(slot)?;
        let mut stmt = self
            .conn
            .prepare("SELECT label FROM key_slots_meta WHERE slot = ?1 LIMIT 1")?;
        let value: Option<String> = stmt
            .query_row(params![i64::from(slot)], |row| row.get(0))
            .optional()?;
        Ok(value.and_then(|text| {
            let trimmed = text.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }))
    }

    /// Set/replace human-readable label for a slot.
    ///
    /// # Errors
    /// 当槽位超出范围、标签为空或 `SQLite` 写入失败时返回错误。
    pub fn set_slot_label(&self, slot: u8, label: &str) -> Result<()> {
        validate_slot(slot)?;
        let trimmed = label.trim();
        if trimmed.is_empty() {
            return Err(AppError::Message("slot label cannot be empty".to_string()));
        }
        let now = now_ts()?;
        self.conn.execute(
            "INSERT INTO key_slots_meta (slot, label, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(slot) DO UPDATE SET
                label = excluded.label,
                updated_at = excluded.updated_at",
            params![i64::from(slot), trimmed, now, now],
        )?;
        Ok(())
    }

    /// Clear label from a slot.
    ///
    /// # Errors
    /// 当槽位超出范围或 `SQLite` 删除失败时返回错误。
    pub fn clear_slot_label(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        self.conn.execute(
            "DELETE FROM key_slots_meta WHERE slot = ?1",
            params![i64::from(slot)],
        )?;
        Ok(())
    }

    /// List all non-empty slot labels.
    ///
    /// # Errors
    /// 当 `SQLite` 查询或迭代失败时返回错误。
    pub fn list_slot_labels(&self) -> Result<Vec<(u8, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT slot, label
             FROM key_slots_meta
             WHERE TRIM(label) <> ''
             ORDER BY slot ASC",
        )?;

        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let slot_i64: i64 = row.get(0)?;
            let label: String = row.get(1)?;
            let Ok(slot) = u8::try_from(slot_i64) else {
                continue;
            };
            if is_valid_slot(slot) {
                out.push((slot, label));
            }
        }
        Ok(out)
    }

    /// Backing sqlite path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Internal helper function.
fn normalize_ui_language(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "zh-cn" => Some(UI_LANG_ZH_CN),
        "en-us" => Some(UI_LANG_EN_US),
        _ => None,
    }
}

/// Validate slot range.
///
/// # Errors
/// 当槽位超出允许范围时返回错误。
pub fn validate_slot(slot: u8) -> Result<()> {
    if is_valid_slot(slot) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "invalid key slot: {slot} (expected {KEY_SLOT_MIN}..={KEY_SLOT_MAX})"
        )))
    }
}

/// Check if slot is inside valid range.
#[must_use]
pub const fn is_valid_slot(slot: u8) -> bool {
    slot <= KEY_SLOT_MAX
}

/// Internal helper function.
fn db_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| AppError::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("awmkit.db");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| AppError::Message("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("awmkit.db");
        Ok(path)
    }
}

/// Internal helper function.
fn open_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS key_slots_meta (
            slot INTEGER PRIMARY KEY,
            label TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )?;
    Ok(conn)
}

/// Internal helper function.
fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::normalize_ui_language;

    #[test]
    fn normalize_supported_ui_language() {
        assert_eq!(normalize_ui_language("zh-CN"), Some("zh-CN"));
        assert_eq!(normalize_ui_language("ZH-cn"), Some("zh-CN"));
        assert_eq!(normalize_ui_language("en-US"), Some("en-US"));
        assert_eq!(normalize_ui_language("en-us"), Some("en-US"));
    }

    #[test]
    fn reject_unsupported_ui_language() {
        assert_eq!(normalize_ui_language("ja-JP"), None);
        assert_eq!(normalize_ui_language(""), None);
        assert_eq!(normalize_ui_language("system"), None);
    }
}
