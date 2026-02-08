use crate::app::error::{AppError, Result};
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
    path: PathBuf,
    conn: Connection,
    entries: Vec<TagEntry>,
}

impl TagStore {
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

    pub fn has_tag(&self, tag: &str) -> bool {
        self.entries.iter().any(|e| e.tag == tag)
    }

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
                return Err(AppError::MappingExists {
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

    pub fn remove(&mut self, username: &str) -> Result<()> {
        let username = normalize_username(username)?;
        let affected = self.conn.execute(
            "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE",
            params![username],
        )?;
        if affected == 0 {
            return Err(AppError::Message(format!("mapping not found: {username}")));
        }
        self.refresh_entries()
    }

    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM tag_mappings", [])?;
        self.refresh_entries()
    }

    pub fn suggest(username: &str) -> Result<Tag> {
        let username = normalize_username(username)?;
        let mut hasher = Sha256::new();
        hasher.update(username.as_bytes());
        let hash = hasher.finalize();
        let identity = hash_to_identity(&hash);
        Tag::new(&identity).map_err(AppError::from)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn refresh_entries(&mut self) -> Result<()> {
        self.entries = load_entries(&self.conn)?;
        Ok(())
    }
}

fn normalize_username(username: &str) -> Result<String> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(AppError::Message("username cannot be empty".to_string()));
    }
    Ok(trimmed.to_string())
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

fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}

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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!("awmkit-tags-{nanos}-{id}.db"));
        path
    }

    #[test]
    fn tag_store_save_list_remove_clear() {
        let path = temp_path();
        let mut store = TagStore::load_at(path.clone()).unwrap();
        let tag = TagStore::suggest("alice").unwrap();
        store.save("alice", &tag, false).unwrap();
        assert_eq!(store.list().len(), 1);

        store.remove("alice").unwrap();
        assert_eq!(store.list().len(), 0);

        store.save("bob", &tag, false).unwrap();
        assert_eq!(store.list().len(), 1);

        store.clear().unwrap();
        assert!(store.list().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tag_store_detects_conflict() {
        let path = temp_path();
        let mut store = TagStore::load_at(path.clone()).unwrap();
        let tag1 = TagStore::suggest("alice").unwrap();
        let tag2 = Tag::new("TESTA").unwrap();

        store.save("alice", &tag1, false).unwrap();
        let err = store.save("alice", &tag2, false).unwrap_err();
        match err {
            AppError::MappingExists { .. } => {}
            _ => panic!("unexpected error"),
        }
        let _ = fs::remove_file(path);
    }
}
