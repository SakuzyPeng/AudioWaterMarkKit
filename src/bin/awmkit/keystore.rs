use crate::error::{CliError, Result};
use keyring::Entry;
use rand::rngs::OsRng;
use rand::RngCore;

const SERVICE: &str = "com.awmkit.watermark";
const USERNAME: &str = "signing-key";

pub const KEY_LEN: usize = 32;

pub struct KeyStore {
    entry: Entry,
}

impl KeyStore {
    pub fn new() -> Result<Self> {
        let entry = Entry::new(SERVICE, USERNAME)
            .map_err(|e| CliError::KeyStore(e.to_string()))?;
        Ok(Self { entry })
    }

    pub fn exists(&self) -> bool {
        self.entry.get_password().is_ok()
    }

    pub fn load(&self) -> Result<Vec<u8>> {
        let hex_key = self
            .entry
            .get_password()
            .map_err(|_| CliError::KeyNotFound)?;
        let key = hex::decode(hex_key)?;
        validate_key_len(key.len())?;
        Ok(key)
    }

    pub fn save(&self, key: &[u8]) -> Result<()> {
        validate_key_len(key.len())?;
        self.entry
            .set_password(&hex::encode(key))
            .map_err(|e| CliError::KeyStore(e.to_string()))?;
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        self.entry
            .delete_password()
            .map_err(|e| CliError::KeyStore(e.to_string()))?;
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
