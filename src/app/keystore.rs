use crate::app::app_settings_store::{validate_slot, AppSettingsStore, KEY_SLOT_MAX, KEY_SLOT_MIN};
use crate::app::error::{AppError, Result};
use keyring::Entry;
use rand::rngs::OsRng;
use rand::RngCore;
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
    pub fn label(&self) -> String {
        match self {
            Self::Keyring => format!("keyring (service: {SERVICE})"),
            #[cfg(windows)]
            Self::Dpapi(path) => format!("dpapi ({})", path.display()),
        }
    }
}

pub struct KeyStore {
    #[cfg(windows)]
    dpapi_base_dir: PathBuf,
}

impl KeyStore {
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

    pub fn active_slot(&self) -> Result<u8> {
        let settings = AppSettingsStore::load()?;
        settings.active_key_slot()
    }

    pub fn exists(&self) -> bool {
        self.active_slot().is_ok_and(|slot| self.exists_slot(slot))
    }

    pub fn exists_slot(&self, slot: u8) -> bool {
        if validate_slot(slot).is_err() {
            return false;
        }
        self.load_slot_with_backend(slot).is_ok()
    }

    pub fn list_configured_slots(&self) -> Vec<u8> {
        (KEY_SLOT_MIN..=KEY_SLOT_MAX)
            .filter(|slot| self.exists_slot(*slot))
            .collect()
    }

    pub fn load(&self) -> Result<Vec<u8>> {
        let slot = self.active_slot()?;
        self.load_slot(slot)
    }

    pub fn load_slot(&self, slot: u8) -> Result<Vec<u8>> {
        self.load_slot_with_backend(slot).map(|(key, _)| key)
    }

    pub fn save(&self, key: &[u8]) -> Result<()> {
        let slot = self.active_slot()?;
        self.save_slot(slot, key)
    }

    pub fn save_slot(&self, slot: u8, key: &[u8]) -> Result<()> {
        validate_slot(slot)?;
        validate_key_len(key.len())?;
        self.save_slot_raw(slot, key)
    }

    pub fn delete(&self) -> Result<()> {
        let slot = self.active_slot()?;
        self.delete_slot(slot)
    }

    pub fn delete_slot(&self, slot: u8) -> Result<()> {
        validate_slot(slot)?;
        let mut removed = false;
        if self.delete_from_keyring_slot(slot).is_ok() {
            removed = true;
        }
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

    pub fn load_with_backend(&self) -> Result<(Vec<u8>, KeyBackend)> {
        let slot = self.active_slot()?;
        self.load_slot_with_backend(slot)
    }

    pub fn load_slot_with_backend(&self, slot: u8) -> Result<(Vec<u8>, KeyBackend)> {
        validate_slot(slot)?;
        match self.load_from_keyring_slot(slot) {
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

    fn save_slot_raw(&self, slot: u8, key: &[u8]) -> Result<()> {
        if self.save_to_keyring_slot(slot, key).is_ok() {
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
        match self.load_from_legacy_keyring() {
            Ok(key) => return Ok(Some(key)),
            Err(AppError::KeyNotFound) | Err(AppError::KeyStore(_)) => {}
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

    fn load_from_keyring_slot(&self, slot: u8) -> Result<Vec<u8>> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        let hex_key = entry.get_password().map_err(|_| AppError::KeyNotFound)?;
        let key = hex::decode(hex_key).map_err(|e| AppError::Message(e.to_string()))?;
        validate_key_len(key.len())?;
        Ok(key)
    }

    fn save_to_keyring_slot(&self, slot: u8, key: &[u8]) -> Result<()> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        entry
            .set_password(&hex::encode(key))
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        Ok(())
    }

    fn delete_from_keyring_slot(&self, slot: u8) -> Result<()> {
        let username = slot_username(slot);
        let entry = keyring_entry(&username)?;
        entry
            .delete_password()
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        Ok(())
    }

    fn load_from_legacy_keyring(&self) -> Result<Vec<u8>> {
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

pub fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    key
}

fn validate_key_len(len: usize) -> Result<()> {
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

fn keyring_entry(username: &str) -> Result<Entry> {
    Entry::new(SERVICE, username).map_err(|e| AppError::KeyStore(e.to_string()))
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
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{LocalFree, BOOL};
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    if data.is_empty() {
        return Err(AppError::KeyStore("dpapi encrypt: empty data".to_string()));
    }

    let mut in_blob = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    let ok: BOOL = unsafe {
        CryptProtectData(
            &mut in_blob,
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };

    if ok == 0 {
        return Err(AppError::KeyStore("dpapi encrypt failed".to_string()));
    }

    let bytes = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let out = bytes.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as *mut std::ffi::c_void);
    }
    Ok(out)
}

#[cfg(windows)]
fn decrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{LocalFree, BOOL};
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    if data.is_empty() {
        return Err(AppError::KeyStore("dpapi decrypt: empty data".to_string()));
    }

    let mut in_blob = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let mut descr = null_mut();

    let ok: BOOL = unsafe {
        CryptUnprotectData(
            &mut in_blob,
            &mut descr,
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };

    if !descr.is_null() {
        unsafe {
            LocalFree(descr as *mut std::ffi::c_void);
        }
    }

    if ok == 0 {
        return Err(AppError::KeyStore("dpapi decrypt failed".to_string()));
    }

    let bytes = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let out = bytes.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as *mut std::ffi::c_void);
    }
    Ok(out)
}
