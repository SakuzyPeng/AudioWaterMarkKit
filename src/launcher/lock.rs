use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Internal constant.
const ACQUIRE_RETRY_COUNT: usize = 300;
/// Internal constant.
const ACQUIRE_RETRY_DELAY_MS: u64 = 100;

/// Internal struct.
pub struct ExtractLock {
    /// Internal field.
    path: PathBuf,
    /// Internal field.
    held: bool,
}

impl ExtractLock {
    /// Internal associated function.
    pub fn acquire(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!("failed to create lock parent dir {}: {e}", parent.display())
            })?;
        }

        for _ in 0..ACQUIRE_RETRY_COUNT {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "pid={}", std::process::id());
                    return Ok(Self {
                        path: path.to_path_buf(),
                        held: true,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    thread::sleep(Duration::from_millis(ACQUIRE_RETRY_DELAY_MS));
                }
                Err(err) => {
                    return Err(format!("failed to create lock {}: {err}", path.display()));
                }
            }
        }

        Err(format!(
            "timed out waiting for extraction lock: {}",
            path.display()
        ))
    }
}

impl Drop for ExtractLock {
    fn drop(&mut self) {
        if self.held {
            let _ = fs::remove_file(&self.path);
            self.held = false;
        }
    }
}
