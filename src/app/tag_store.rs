use crate::app::error::{AppError, Result};
use crate::charset::CHARSET;
use crate::Tag;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone)]
pub struct TagEntry {
    pub username: String,
    pub tag: String,
    pub created_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct TagStoreData {
    version: u8,
    entries: Vec<TagEntry>,
}

pub struct TagStore {
    path: PathBuf,
    data: TagStoreData,
}

impl TagStore {
    pub fn load() -> Result<Self> {
        let path = tags_path()?;
        let data = load_store(&path)?;
        Ok(Self { path, data })
    }

    #[cfg(test)]
    fn load_at(path: PathBuf) -> Result<Self> {
        let data = load_store(&path)?;
        Ok(Self { path, data })
    }

    pub fn list(&self) -> &[TagEntry] {
        &self.data.entries
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.data.entries.iter().any(|e| e.tag == tag)
    }

    pub fn save(&mut self, username: &str, tag: &Tag, force: bool) -> Result<()> {
        let username = normalize_username(username)?;
        let tag_str = tag.as_str().to_string();

        if let Some(existing) = self
            .data
            .entries
            .iter_mut()
            .find(|entry| entry.username == username)
        {
            if existing.tag == tag_str {
                return Ok(());
            }
            if !force {
                return Err(AppError::MappingExists {
                    username,
                    existing_tag: existing.tag.clone(),
                });
            }
            existing.tag = tag_str;
            existing.created_at = now_ts()?;
        } else {
            self.data.entries.push(TagEntry {
                username,
                tag: tag_str,
                created_at: now_ts()?,
            });
        }

        self.data.version = 1;
        self.data
            .entries
            .sort_by(|a, b| a.username.cmp(&b.username));
        self.persist()
    }

    pub fn remove(&mut self, username: &str) -> Result<()> {
        let username = normalize_username(username)?;
        let before = self.data.entries.len();
        self.data.entries.retain(|entry| entry.username != username);
        if self.data.entries.len() == before {
            return Err(AppError::Message(format!("mapping not found: {username}")));
        }
        self.persist()
    }

    pub fn clear(&mut self) -> Result<()> {
        self.data.entries.clear();
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
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

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, data)?;
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

fn tags_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| AppError::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("tags.json");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| AppError::Message("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("tags.json");
        Ok(path)
    }
}

fn load_store(path: &Path) -> Result<TagStoreData> {
    if !path.exists() {
        return Ok(TagStoreData::default());
    }
    let raw = fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(TagStoreData::default());
    }
    let store: TagStoreData = serde_json::from_str(&raw)?;
    Ok(store)
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
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("awmkit-tags-{nanos}.json"));
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
        assert!(!path.exists());
    }

    #[test]
    fn tag_store_detects_conflict() {
        let path = temp_path();
        let mut store = TagStore::load_at(path).unwrap();
        let tag1 = TagStore::suggest("alice").unwrap();
        let tag2 = Tag::new("TESTA").unwrap();

        store.save("alice", &tag1, false).unwrap();
        let err = store.save("alice", &tag2, false).unwrap_err();
        match err {
            AppError::MappingExists { .. } => {}
            _ => panic!("unexpected error"),
        }
    }
}
