use crate::app::app_settings_store::{validate_slot, AppSettingsStore, KEY_SLOT_MAX, KEY_SLOT_MIN};
use crate::app::error::{AppError, Result};
use crate::app::{EvidenceSlotUsage, EvidenceStore};
use keyring::Entry;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
#[cfg(windows)]
use std::path::PathBuf;

const SERVICE: &str = "com.awmkit.watermark";
const LEGACY_USERNAME: &str = "signing-key";
const SLOT_USERNAME_PREFIX: &str = "signing-key-slot-";

pub const KEY_LEN: usize = 32;

#[derive(Debug, Clone)]
pub enum KeyBackend {
    Keyring,
    #[cfg(windows)]
    Dpapi(PathBuf),
}

impl KeyBackend {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Keyring => format!("keyring (service: {SERVICE})"),
            #[cfg(windows)]
            Self::Dpapi(path) => format!("dpapi ({})", path.display()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KeySlotSummary {
    pub slot: u8,
    pub is_active: bool,
    pub has_key: bool,
    pub key_id: Option<String>,
    pub label: Option<String>,
    pub evidence_count: usize,
    pub last_evidence_at: Option<u64>,
    pub status_text: String,
    pub duplicate_of_slots: Vec<u8>,
}

pub struct KeyStore {
    #[cfg(windows)]
    dpapi_base_dir: PathBuf,
}

impl KeyStore {
    /// # Errors
    /// 当配置存储初始化、旧密钥迁移或平台密钥后端初始化失败时返回错误。
    pub fn new() -> Result<Self> {
        #[cfg(not(windows))]
        {
            let store = Self {};
            // Ensure settings table exists and perform one-time legacy migration.
            let _ = AppSettingsStore::load()?;
            store.migrate_legacy_to_slot0()?;
            Ok(store)
        }

        #[cfg(windows)]
        {
            let dpapi_base_dir = dpapi_base_dir()?;
            let store = Self { dpapi_base_dir };
            let _ = AppSettingsStore::load()?;
            store.migrate_legacy_to_slot0()?;
            Ok(store)
        }
    }

    /// # Errors
    /// 当配置存储读取失败时返回错误。
    pub fn active_slot(&self) -> Result<u8> {
        let settings = AppSettingsStore::load()?;
        settings.active_key_slot()
    }

    /// # Errors
    /// 当槽位非法或配置写入失败时返回错误。
    pub fn set_active_slot(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        let settings = AppSettingsStore::load()?;
        settings.set_active_key_slot(slot)
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        self.active_slot().is_ok_and(|slot| self.exists_slot(slot))
    }

    #[must_use]
    pub fn exists_slot(&self, slot: u8) -> bool {
        if validate_slot(slot).is_err() {
            return false;
        }
        self.load_slot_with_backend(slot).is_ok()
    }

    #[must_use]
    pub fn list_configured_slots(&self) -> Vec<u8> {
        (KEY_SLOT_MIN..=KEY_SLOT_MAX)
            .filter(|slot| self.exists_slot(*slot))
            .collect()
    }

    /// # Errors
    /// 当当前活动槽位读取失败或该槽位密钥读取失败时返回错误。
    pub fn load(&self) -> Result<Vec<u8>> {
        let slot = self.active_slot()?;
        self.load_slot(slot)
    }

    /// # Errors
    /// 当槽位非法或底层密钥后端读取失败时返回错误。
    pub fn load_slot(&self, slot: u8) -> Result<Vec<u8>> {
        self.load_slot_with_backend(slot).map(|(key, _)| key)
    }

    /// # Errors
    /// 当活动槽位读取失败或密钥保存失败时返回错误。
    pub fn save(&self, key: &[u8]) -> Result<()> {
        let slot = self.active_slot()?;
        self.save_slot(slot, key)
    }

    /// # Errors
    /// 当槽位非法、密钥长度不合法或写入后端失败时返回错误。
    pub fn save_slot(&self, slot: u8, key: &[u8]) -> Result<()> {
        validate_slot(slot)?;
        validate_key_len(key.len())?;
        self.save_slot_raw(slot, key)
    }

    /// # Errors
    /// 当活动槽位读取失败或删除失败时返回错误。
    pub fn delete(&self) -> Result<()> {
        let slot = self.active_slot()?;
        self.delete_slot(slot)
    }

    /// # Errors
    /// 当槽位非法、对应密钥不存在或删除后端失败时返回错误。
    pub fn delete_slot(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        #[cfg(not(windows))]
        let removed = Self::delete_from_keyring_slot(slot).is_ok();

        #[cfg(windows)]
        let mut removed = Self::delete_from_keyring_slot(slot).is_ok();
        #[cfg(windows)]
        {
            let slot_path = self.dpapi_slot_path(slot);
            if slot_path.is_file() {
                std::fs::remove_file(slot_path)?;
                removed = true;
            }
        }
        if !removed {
            return Err(AppError::KeyNotFound);
        }
        Ok(())
    }

    /// Delete key material from a slot and reconcile active slot if needed.
    ///
    /// Returns the effective active slot after deletion.
    ///
    /// # Errors
    /// 当槽位非法、删除失败或活动槽位重设失败时返回错误。
    pub fn delete_slot_and_reconcile_active(&self, slot: u8) -> Result<u8> {
        validate_slot(slot)?;
        let active_before = self.active_slot()?;
        self.delete_slot(slot)?;

        if slot != active_before {
            return Ok(active_before);
        }

        let fallback_slot = self.fallback_active_slot();
        self.set_active_slot(fallback_slot)?;
        Ok(fallback_slot)
    }

    /// # Errors
    /// 当活动槽位读取失败或密钥后端读取失败时返回错误。
    pub fn load_with_backend(&self) -> Result<(Vec<u8>, KeyBackend)> {
        let slot = self.active_slot()?;
        self.load_slot_with_backend(slot)
    }

    /// # Errors
    /// 当槽位非法，且 keyring 与平台回退后端都读取失败时返回错误。
    pub fn load_slot_with_backend(&self, slot: u8) -> Result<(Vec<u8>, KeyBackend)> {
        validate_slot(slot)?;
        match Self::load_from_keyring_slot(slot) {
            Ok(key) => Ok((key, KeyBackend::Keyring)),
            Err(keyring_err) => {
                #[cfg(windows)]
                {
                    if let Some(key) = self.load_from_dpapi_slot(slot)? {
                        return Ok((key, KeyBackend::Dpapi(self.dpapi_slot_path(slot))));
                    }
                }
                Err(keyring_err)
            }
        }
    }

    /// Build full slot summaries for UI presentation.
    ///
    /// # Errors
    /// 当任一槽位的配置、证据统计或密钥读取失败时返回错误。
    pub fn slot_summaries(&self) -> Result<Vec<KeySlotSummary>> {
        let active_slot = self.active_slot()?;
        let settings = AppSettingsStore::load()?;
        let evidence_store = EvidenceStore::load()?;
        let mut summaries = Vec::with_capacity(usize::from(KEY_SLOT_MAX) + 1);

        for slot in KEY_SLOT_MIN..=KEY_SLOT_MAX {
            let key = self.load_slot(slot).ok();
            let key_id = key.as_ref().map(|bytes| key_id_from_key_material(bytes));
            let label = settings.slot_label(slot)?;
            let usage = if let Some(key_id) = key_id.as_deref() {
                evidence_store.usage_by_slot_and_key_id(slot, key_id)?
            } else {
                EvidenceSlotUsage {
                    count: 0,
                    last_created_at: None,
                }
            };
            let has_key = key.is_some();

            let status_text = if !has_key {
                "empty".to_string()
            } else if slot == active_slot {
                "active".to_string()
            } else {
                "configured".to_string()
            };

            summaries.push(KeySlotSummary {
                slot,
                is_active: slot == active_slot,
                has_key,
                key_id,
                label,
                evidence_count: usage.count,
                last_evidence_at: usage.last_created_at,
                status_text,
                duplicate_of_slots: Vec::new(),
            });
        }

        apply_duplicate_status(&mut summaries);
        Ok(summaries)
    }

    fn save_slot_raw(&self, slot: u8, key: &[u8]) -> Result<()> {
        #[cfg(not(windows))]
        let _ = self;
        if Self::save_to_keyring_slot(slot, key).is_ok() {
            return Ok(());
        }
        #[cfg(windows)]
        {
            self.save_to_dpapi_slot(slot, key)?;
            return Ok(());
        }
        #[cfg(not(windows))]
        {
            Err(AppError::KeyStore(
                "failed to store key in keyring".to_string(),
            ))
        }
    }

    fn fallback_active_slot(&self) -> u8 {
        if self.exists_slot(KEY_SLOT_MIN) {
            return KEY_SLOT_MIN;
        }
        self.list_configured_slots()
            .into_iter()
            .min()
            .unwrap_or(KEY_SLOT_MIN)
    }

    fn migrate_legacy_to_slot0(&self) -> Result<()> {
        if self.exists_slot(KEY_SLOT_MIN) {
            return Ok(());
        }
        if let Some(legacy_key) = self.try_load_legacy_key()? {
            self.save_slot_raw(KEY_SLOT_MIN, &legacy_key)?;
        }
        Ok(())
    }

    fn try_load_legacy_key(&self) -> Result<Option<Vec<u8>>> {
        #[cfg(not(windows))]
        let _ = self;
        match Self::load_from_legacy_keyring() {
            Ok(key) => return Ok(Some(key)),
            Err(AppError::KeyNotFound | AppError::KeyStore(_)) => {}
            Err(err) => return Err(err),
        }
        #[cfg(windows)]
        {
            if let Some(key) = self.load_from_legacy_dpapi()? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }

    fn load_from_keyring_slot(slot: u8) -> Result<Vec<u8>> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        let hex_key = entry.get_password().map_err(|_| AppError::KeyNotFound)?;
        let key = hex::decode(hex_key).map_err(|e| AppError::Message(e.to_string()))?;
        validate_key_len(key.len())?;
        Ok(key)
    }

    fn save_to_keyring_slot(slot: u8, key: &[u8]) -> Result<()> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        entry
            .set_password(&hex::encode(key))
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        Ok(())
    }

    fn delete_from_keyring_slot(slot: u8) -> Result<()> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        entry
            .delete_password()
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        Ok(())
    }

    fn load_from_legacy_keyring() -> Result<Vec<u8>> {
        let entry = keyring_entry(LEGACY_USERNAME)?;
        let hex_key = entry.get_password().map_err(|_| AppError::KeyNotFound)?;
        let key = hex::decode(hex_key).map_err(|e| AppError::Message(e.to_string()))?;
        validate_key_len(key.len())?;
        Ok(key)
    }

    #[cfg(windows)]
    fn load_from_dpapi_slot(&self, slot: u8) -> Result<Option<Vec<u8>>> {
        let path = self.dpapi_slot_path(slot);
        if !path.is_file() {
            return Ok(None);
        }
        let encrypted = std::fs::read(path)?;
        let decrypted = decrypt_dpapi(&encrypted)?;
        validate_key_len(decrypted.len())?;
        Ok(Some(decrypted))
    }

    #[cfg(windows)]
    fn save_to_dpapi_slot(&self, slot: u8, key: &[u8]) -> Result<()> {
        let path = self.dpapi_slot_path(slot);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encrypted = encrypt_dpapi(key)?;
        std::fs::write(path, encrypted)?;
        Ok(())
    }

    #[cfg(windows)]
    fn load_from_legacy_dpapi(&self) -> Result<Option<Vec<u8>>> {
        let path = self.dpapi_legacy_path();
        if !path.is_file() {
            return Ok(None);
        }
        let encrypted = std::fs::read(path)?;
        let decrypted = decrypt_dpapi(&encrypted)?;
        validate_key_len(decrypted.len())?;
        Ok(Some(decrypted))
    }

    #[cfg(windows)]
    fn dpapi_slot_path(&self, slot: u8) -> PathBuf {
        let mut path = self.dpapi_base_dir.clone();
        path.push("keys");
        path.push(format!("slot-{slot}.dpapi"));
        path
    }

    #[cfg(windows)]
    fn dpapi_legacy_path(&self) -> PathBuf {
        let mut path = self.dpapi_base_dir.clone();
        path.push("key.dpapi");
        path
    }
}

#[must_use]
pub fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    key
}

const fn validate_key_len(len: usize) -> Result<()> {
    if len == KEY_LEN {
        Ok(())
    } else {
        Err(AppError::InvalidKeyLength {
            expected: KEY_LEN,
            actual: len,
        })
    }
}

fn slot_username(slot: u8) -> String {
    format!("{SLOT_USERNAME_PREFIX}{slot}")
}

#[must_use]
pub fn key_id_from_key_material(key: &[u8]) -> String {
    let digest = Sha256::digest(key);
    hex::encode_upper(digest)[..10].to_string()
}

fn apply_duplicate_status(summaries: &mut [KeySlotSummary]) {
    let mut buckets: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    for summary in summaries.iter() {
        if let Some(key_id) = summary.key_id.as_ref() {
            buckets
                .entry(key_id.clone())
                .or_default()
                .push(summary.slot);
        }
    }

    for summary in summaries.iter_mut() {
        let Some(key_id) = summary.key_id.as_ref() else {
            continue;
        };
        let Some(slots) = buckets.get(key_id) else {
            continue;
        };
        if slots.len() <= 1 {
            continue;
        }

        summary.status_text = "duplicate".to_string();
        summary.duplicate_of_slots = slots
            .iter()
            .copied()
            .filter(|slot| *slot != summary.slot)
            .collect();
    }
}

fn keyring_entry(username: &str) -> Result<Entry> {
    Entry::new(SERVICE, username).map_err(|e| AppError::KeyStore(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{apply_duplicate_status, key_id_from_key_material, KeySlotSummary};

    #[test]
    fn key_id_is_stable_and_ten_chars() {
        let key = [42u8; 32];
        let first = key_id_from_key_material(&key);
        let second = key_id_from_key_material(&key);
        assert_eq!(first, second);
        assert_eq!(first.len(), 10);
    }

    #[test]
    fn duplicate_status_marks_related_slots() {
        let mut rows = vec![
            KeySlotSummary {
                slot: 0,
                is_active: true,
                has_key: true,
                key_id: Some("AAAAAAAAAA".to_string()),
                label: None,
                evidence_count: 0,
                last_evidence_at: None,
                status_text: "active".to_string(),
                duplicate_of_slots: Vec::new(),
            },
            KeySlotSummary {
                slot: 2,
                is_active: false,
                has_key: true,
                key_id: Some("AAAAAAAAAA".to_string()),
                label: None,
                evidence_count: 0,
                last_evidence_at: None,
                status_text: "configured".to_string(),
                duplicate_of_slots: Vec::new(),
            },
            KeySlotSummary {
                slot: 1,
                is_active: false,
                has_key: false,
                key_id: None,
                label: None,
                evidence_count: 0,
                last_evidence_at: None,
                status_text: "empty".to_string(),
                duplicate_of_slots: Vec::new(),
            },
        ];

        apply_duplicate_status(&mut rows);

        assert_eq!(rows[0].status_text, "duplicate");
        assert_eq!(rows[1].status_text, "duplicate");
        assert_eq!(rows[2].status_text, "empty");
        assert_eq!(rows[0].duplicate_of_slots, vec![2]);
        assert_eq!(rows[1].duplicate_of_slots, vec![0]);
    }
}

#[cfg(windows)]
fn dpapi_base_dir() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .ok_or_else(|| AppError::KeyStore("LOCALAPPDATA/APPDATA not set".to_string()))?;
    let mut path = PathBuf::from(base);
    path.push("awmkit");
    Ok(path)
}

#[cfg(windows)]
fn encrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
    use windows_dpapi::{encrypt_data, Scope};

    if data.is_empty() {
        return Err(AppError::KeyStore("dpapi encrypt: empty data".to_string()));
    }

    encrypt_data(data, Scope::User)
        .map_err(|e| AppError::KeyStore(format!("dpapi encrypt failed: {e}")))
}

#[cfg(windows)]
fn decrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
    use windows_dpapi::{decrypt_data, Scope};

    if data.is_empty() {
        return Err(AppError::KeyStore("dpapi decrypt: empty data".to_string()));
    }

    decrypt_data(data, Scope::User)
        .map_err(|e| AppError::KeyStore(format!("dpapi decrypt failed: {e}")))
}
