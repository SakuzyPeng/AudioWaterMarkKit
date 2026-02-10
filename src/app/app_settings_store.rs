use crate::app::error::{AppError, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimum valid key slot.
pub const KEY_SLOT_MIN: u8 = 0;
/// Maximum valid key slot.
pub const KEY_SLOT_MAX: u8 = 31;
const ACTIVE_KEY_SLOT_KEY: &str = "active_key_slot";

/// App-level settings store backed by sqlite.
pub struct AppSettingsStore {
    conn: Connection,
    path: PathBuf,
}

impl AppSettingsStore {
    /// Open settings store at shared awmkit sqlite database.
    pub fn load() -> Result<Self> {
        let path = db_path()?;
        let conn = open_db(&path)?;
        Ok(Self { conn, path })
    }

    /// Read active key slot. Missing/invalid value falls back to 0.
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

    /// Get human-readable label for a slot.
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
    pub fn clear_slot_label(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        self.conn.execute(
            "DELETE FROM key_slots_meta WHERE slot = ?1",
            params![i64::from(slot)],
        )?;
        Ok(())
    }

    /// List all non-empty slot labels.
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

/// Validate slot range.
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

fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}
