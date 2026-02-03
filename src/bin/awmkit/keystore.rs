use crate::error::{CliError, Result};
use keyring::Entry;
use rand::rngs::OsRng;
use rand::RngCore;
use std::path::PathBuf;

const SERVICE: &str = "com.awmkit.watermark";
const USERNAME: &str = "signing-key";

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
    #[cfg(not(windows))]
    entry: Entry,
    #[cfg(windows)]
    entry: Option<Entry>,
    #[cfg(windows)]
    dpapi_path: PathBuf,
}

impl KeyStore {
    pub fn new() -> Result<Self> {
        #[cfg(not(windows))]
        {
            let entry =
                Entry::new(SERVICE, USERNAME).map_err(|e| CliError::KeyStore(e.to_string()))?;
            Ok(Self { entry })
        }

        #[cfg(windows)]
        {
            let entry = Entry::new(SERVICE, USERNAME).ok();
            let dpapi_path = dpapi_path()?;
            Ok(Self { entry, dpapi_path })
        }
    }

    pub fn exists(&self) -> bool {
        if self.load_from_keyring().is_ok() {
            return true;
        }
        #[cfg(windows)]
        {
            return self.dpapi_path.is_file();
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn load(&self) -> Result<Vec<u8>> {
        self.load_with_backend().map(|(key, _)| key)
    }

    pub fn save(&self, key: &[u8]) -> Result<()> {
        validate_key_len(key.len())?;
        if let Ok(()) = self.save_to_keyring(key) {
            return Ok(());
        }
        #[cfg(windows)]
        {
            self.save_to_dpapi(key)?;
            return Ok(());
        }
        #[cfg(not(windows))]
        {
            Err(CliError::KeyStore("failed to store key in keyring".to_string()))
        }
    }

    pub fn delete(&self) -> Result<()> {
        let mut removed = false;
        if self.delete_from_keyring().is_ok() {
            removed = true;
        }
        #[cfg(windows)]
        {
            if self.dpapi_path.is_file() {
                std::fs::remove_file(&self.dpapi_path)?;
                removed = true;
            }
        }
        if !removed {
            return Err(CliError::KeyNotFound);
        }
        Ok(())
    }

    pub fn load_with_backend(&self) -> Result<(Vec<u8>, KeyBackend)> {
        match self.load_from_keyring() {
            Ok(key) => Ok((key, KeyBackend::Keyring)),
            Err(err) => {
                #[cfg(windows)]
                {
                    if let Some(key) = self.load_from_dpapi()? {
                        return Ok((key, KeyBackend::Dpapi(self.dpapi_path.clone())));
                    }
                }
                Err(err)
            }
        }
    }

    fn load_from_keyring(&self) -> Result<Vec<u8>> {
        #[cfg(not(windows))]
        {
            let hex_key = self
                .entry
                .get_password()
                .map_err(|_| CliError::KeyNotFound)?;
            let key = hex::decode(hex_key)?;
            validate_key_len(key.len())?;
            Ok(key)
        }

        #[cfg(windows)]
        {
            let entry = self
                .entry
                .as_ref()
                .ok_or_else(|| CliError::KeyStore("keyring unavailable".to_string()))?;
            let hex_key = entry
                .get_password()
                .map_err(|_| CliError::KeyNotFound)?;
            let key = hex::decode(hex_key)?;
            validate_key_len(key.len())?;
            Ok(key)
        }
    }

    fn save_to_keyring(&self, key: &[u8]) -> Result<()> {
        #[cfg(not(windows))]
        {
            self.entry
                .set_password(&hex::encode(key))
                .map_err(|e| CliError::KeyStore(e.to_string()))?;
            Ok(())
        }

        #[cfg(windows)]
        {
            let entry = self
                .entry
                .as_ref()
                .ok_or_else(|| CliError::KeyStore("keyring unavailable".to_string()))?;
            entry
                .set_password(&hex::encode(key))
                .map_err(|e| CliError::KeyStore(e.to_string()))?;
            Ok(())
        }
    }

    fn delete_from_keyring(&self) -> Result<()> {
        #[cfg(not(windows))]
        {
            self.entry
                .delete_password()
                .map_err(|e| CliError::KeyStore(e.to_string()))?;
            Ok(())
        }

        #[cfg(windows)]
        {
            let entry = self
                .entry
                .as_ref()
                .ok_or_else(|| CliError::KeyStore("keyring unavailable".to_string()))?;
            entry
                .delete_password()
                .map_err(|e| CliError::KeyStore(e.to_string()))?;
            Ok(())
        }
    }

    #[cfg(windows)]
    fn load_from_dpapi(&self) -> Result<Option<Vec<u8>>> {
        if !self.dpapi_path.is_file() {
            return Ok(None);
        }
        let encrypted = std::fs::read(&self.dpapi_path)?;
        let decrypted = decrypt_dpapi(&encrypted)?;
        validate_key_len(decrypted.len())?;
        Ok(Some(decrypted))
    }

    #[cfg(windows)]
    fn save_to_dpapi(&self, key: &[u8]) -> Result<()> {
        if let Some(parent) = self.dpapi_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let encrypted = encrypt_dpapi(key)?;
        std::fs::write(&self.dpapi_path, encrypted)?;
        Ok(())
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
        Err(CliError::InvalidKeyLength {
            expected: KEY_LEN,
            actual: len,
        })
    }
}

#[cfg(windows)]
fn dpapi_path() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .ok_or_else(|| CliError::KeyStore("LOCALAPPDATA/APPDATA not set".to_string()))?;
    let mut path = PathBuf::from(base);
    path.push("awmkit");
    path.push("key.dpapi");
    Ok(path)
}

#[cfg(windows)]
fn encrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{BOOL, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    if data.is_empty() {
        return Err(CliError::KeyStore("dpapi encrypt: empty data".to_string()));
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
        return Err(CliError::KeyStore("dpapi encrypt failed".to_string()));
    }

    let bytes = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let out = bytes.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as isize);
    }
    Ok(out)
}

#[cfg(windows)]
fn decrypt_dpapi(data: &[u8]) -> Result<Vec<u8>> {
    use std::ptr::{null_mut, null};
    use windows_sys::Win32::Foundation::{BOOL, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    if data.is_empty() {
        return Err(CliError::KeyStore("dpapi decrypt: empty data".to_string()));
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
            LocalFree(descr as isize);
        }
    }

    if ok == 0 {
        return Err(CliError::KeyStore("dpapi decrypt failed".to_string()));
    }

    let bytes = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
    let out = bytes.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as isize);
    }
    Ok(out)
}
