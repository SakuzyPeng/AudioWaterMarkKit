use crate::app::error::{Failure, Result};
use crate::charset::CHARSET;
use crate::Tag;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct TagEntry {
    pub username: String,
    pub tag: String,
    pub created_at: u64,
}

pub struct TagStore {
    /// Internal field.
    path: PathBuf,
    /// Internal field.
    conn: Connection,
    /// Internal field.
    entries: Vec<TagEntry>,
}

impl TagStore {
    /// # Errors
    /// 当数据库路径解析、目录创建、`SQLite` 打开或初始读取失败时返回错误。.
    pub fn load() -> Result<Self> {
        let path = db_path()?;
        let conn = open_db(&path)?;
        let entries = load_entries(&conn)?;
        Ok(Self {
            path,
            conn,
            entries,
        })
    }

    #[cfg(test)]
    fn load_at(path: PathBuf) -> Result<Self> {
        let conn = open_db(&path)?;
        let entries = load_entries(&conn)?;
        Ok(Self {
            path,
            conn,
            entries,
        })
    }

    pub fn list(&self) -> &[TagEntry] {
        &self.entries
    }

    /// List mappings with an upper bound.
    pub fn list_recent(&self, limit: usize) -> Vec<TagEntry> {
        let normalized = limit.max(1);
        self.entries.iter().take(normalized).cloned().collect()
    }

    /// Get mapping tag by username (case-insensitive).
    ///
    /// # Errors
    /// 当用户名非法或 `SQLite` 查询失败时返回错误。.
    pub fn lookup_tag_ci(&self, username: &str) -> Result<Option<String>> {
        let username = normalize_username(username)?;
        let tag = self
            .conn
            .query_row(
                "SELECT tag FROM tag_mappings WHERE username = ?1 COLLATE NOCASE LIMIT 1",
                params![username],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(tag)
    }

    /// Save only if username does not exist.
    /// Returns true when inserted, false when mapping already exists.
    ///
    /// # Errors
    /// 当用户名非法、时间戳获取失败或 `SQLite` 写入失败时返回错误。.
    pub fn save_if_absent(&mut self, username: &str, tag: &Tag) -> Result<bool> {
        let username = normalize_username(username)?;
        if self.lookup_tag_ci(&username)?.is_some() {
            return Ok(false);
        }
        let now = now_ts()?;
        self.conn.execute(
            "INSERT INTO tag_mappings (username, tag, created_at) VALUES (?1, ?2, ?3)",
            params![username, tag.as_str(), now],
        )?;
        self.refresh_entries()?;
        Ok(true)
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.entries.iter().any(|e| e.tag == tag)
    }

    /// # Errors
    /// 当用户名非法、时间戳获取失败、冲突未强制覆盖或 `SQLite` 写入失败时返回错误。.
    pub fn save(&mut self, username: &str, tag: &Tag, force: bool) -> Result<()> {
        let username = normalize_username(username)?;
        let tag_str = tag.as_str().to_string();
        let existing = self
            .conn
            .query_row(
                "SELECT username, tag FROM tag_mappings WHERE username = ?1 COLLATE NOCASE LIMIT 1",
                params![username],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        let now = now_ts()?;
        if let Some((existing_username, existing_tag)) = existing {
            if existing_tag == tag_str {
                return Ok(());
            }
            if !force {
                return Err(Failure::MappingExists {
                    username,
                    existing_tag,
                });
            }
            self.conn.execute(
                "UPDATE tag_mappings
                 SET username = ?1, tag = ?2, created_at = ?3
                 WHERE username = ?4 COLLATE NOCASE",
                params![username, tag_str, now, existing_username],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO tag_mappings (username, tag, created_at) VALUES (?1, ?2, ?3)",
                params![username, tag_str, now],
            )?;
        }

        self.refresh_entries()
    }

    /// # Errors
    /// 当用户名非法、映射不存在或 `SQLite` 删除失败时返回错误。.
    pub fn remove(&mut self, username: &str) -> Result<()> {
        let username = normalize_username(username)?;
        let affected = self.conn.execute(
            "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE",
            params![username],
        )?;
        if affected == 0 {
            return Err(Failure::Message(format!("mapping not found: {username}")));
        }
        self.refresh_entries()
    }

    /// # Errors
    /// 当 `SQLite` 删除或刷新内存缓存失败时返回错误。.
    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM tag_mappings", [])?;
        self.refresh_entries()
    }

    /// Remove multiple usernames (case-insensitive).
    /// Returns deleted row count.
    ///
    /// # Errors
    /// 当任一用户名非法或 `SQLite` 删除失败时返回错误。.
    pub fn remove_usernames(&mut self, usernames: &[String]) -> Result<usize> {
        let mut deleted = 0usize;
        for username in usernames {
            let normalized = normalize_username(username)?;
            let affected = self.conn.execute(
                "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE",
                params![normalized],
            )?;
            deleted += affected;
        }
        if deleted > 0 {
            self.refresh_entries()?;
        }
        Ok(deleted)
    }

    /// Count mapping rows.
    pub const fn count(&self) -> usize {
        self.entries.len()
    }

    /// # Errors
    /// 当用户名非法或构造 `Tag` 失败时返回错误。.
    pub fn suggest(username: &str) -> Result<Tag> {
        let username = normalize_username(username)?;
        let mut hasher = Sha256::new();
        hasher.update(username.as_bytes());
        let hash = hasher.finalize();
        let identity = hash_to_identity(&hash);
        Tag::new(&identity).map_err(Failure::from)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Internal helper method.
    fn refresh_entries(&mut self) -> Result<()> {
        self.entries = load_entries(&self.conn)?;
        Ok(())
    }
}

/// Internal helper function.
fn normalize_username(username: &str) -> Result<String> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(Failure::Message("username cannot be empty".to_string()));
    }
    Ok(trimmed.to_string())
}

/// Internal helper function.
fn db_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| Failure::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("awmkit.db");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home =
            std::env::var_os("HOME").ok_or_else(|| Failure::Message("HOME not set".to_string()))?;
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
        "CREATE TABLE IF NOT EXISTS tag_mappings (
            username TEXT NOT NULL COLLATE NOCASE PRIMARY KEY,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tag_mappings_created_at
        ON tag_mappings(created_at DESC);",
    )?;
    Ok(conn)
}

/// Internal helper function.
fn load_entries(conn: &Connection) -> Result<Vec<TagEntry>> {
    let mut stmt = conn.prepare(
        "SELECT username, tag, created_at
         FROM tag_mappings
         ORDER BY username COLLATE NOCASE ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(TagEntry {
            username: row.get(0)?,
            tag: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// Internal helper function.
fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Failure::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}

/// Internal helper function.
fn hash_to_identity(hash: &[u8]) -> String {
    let mut out = String::with_capacity(7);
    let mut acc: u64 = 0;
    let mut acc_bits: u8 = 0;

    for &b in hash {
        acc = (acc << 8) | u64::from(b);
        acc_bits += 8;
        while acc_bits >= 5 && out.len() < 7 {
            let shift = acc_bits - 5;
            let idx = ((acc >> shift) & 0x1F) as usize;
            out.push(CHARSET[idx] as char);
            acc_bits -= 5;
        }
        if out.len() >= 7 {
            break;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    macro_rules! ok_or_return {
        ($expr:expr) => {{
            let result = $expr;
            assert!(result.is_ok());
            let Ok(value) = result else {
                return;
            };
            value
        }};
    }

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!("awmkit-tags-{nanos}-{id}.db"));
        path
    }

    #[test]
    fn tag_store_save_list_remove_clear() {
        let path = temp_path();
        let mut store = ok_or_return!(TagStore::load_at(path.clone()));
        let tag = ok_or_return!(TagStore::suggest("alice"));
        assert!(store.save("alice", &tag, false).is_ok());
        assert_eq!(store.list().len(), 1);

        assert!(store.remove("alice").is_ok());
        assert_eq!(store.list().len(), 0);

        assert!(store.save("bob", &tag, false).is_ok());
        assert_eq!(store.list().len(), 1);

        assert!(store.clear().is_ok());
        assert!(store.list().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tag_store_detects_conflict() {
        let path = temp_path();
        let mut store = ok_or_return!(TagStore::load_at(path.clone()));
        let tag1 = ok_or_return!(TagStore::suggest("alice"));
        let tag2 = ok_or_return!(Tag::new("TESTA"));

        assert!(store.save("alice", &tag1, false).is_ok());
        let err = store.save("alice", &tag2, false);
        assert!(err.is_err());
        let Some(err) = err.err() else {
            return;
        };
        assert!(matches!(err, Failure::MappingExists { .. }));
        let _ = fs::remove_file(path);
    }
}
