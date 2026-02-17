use crate::error::{Error, Result};
#[cfg(feature = "bundled")]
use std::fs::{self, File};
#[cfg(feature = "bundled")]
use std::io::Write;
#[cfg(feature = "bundled")]
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "bundled")]
use sha2::{Digest, Sha256};

#[cfg(all(feature = "bundled", target_os = "windows"))]
const BIN_REL: &str = "bin/audiowmark.exe";

#[cfg(all(feature = "bundled", target_os = "macos"))]
/// Internal constant.
const BIN_REL: &str = "bin/audiowmark";

#[cfg(all(
    feature = "bundled",
    not(target_os = "windows"),
    not(target_os = "macos")
))]
const BIN_REL: &str = "";

#[cfg(all(
    feature = "bundled",
    not(any(target_os = "windows", target_os = "macos"))
))]
compile_error!("bundled feature is only supported on windows and macos.");

#[cfg(all(feature = "bundled", target_os = "windows"))]
const BUNDLE_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/bundled/audiowmark-windows-x86_64.zip"
));

#[cfg(all(feature = "bundled", target_os = "macos"))]
/// Internal constant.
const BUNDLE_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/bundled/audiowmark-macos-arm64.zip"
));

/// Internal helper function.
#[cfg(feature = "bundled")]
pub fn ensure_extracted() -> Result<PathBuf> {
    let cache_root = cache_root()?;
    let bin_path = cache_root.join(BIN_REL);
    let marker_path = cache_root.join("bundle.sha256");

    let expected = bundle_hash();
    if bin_path.is_file() {
        if let Ok(current) = fs::read_to_string(&marker_path) {
            if current.trim() == expected {
                return Ok(bin_path);
            }
        }
    }

    if cache_root.exists() {
        fs::remove_dir_all(&cache_root)?;
    }
    fs::create_dir_all(&cache_root)?;

    extract_zip(&cache_root)?;

    if let Some(parent) = bin_path.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    ensure_executable(&bin_path)?;

    let mut marker = File::create(&marker_path)?;
    marker.write_all(expected.as_bytes())?;

    Ok(bin_path)
}

/// Internal helper function.
#[cfg(feature = "bundled")]
fn bundle_hash() -> String {
    #[cfg(feature = "bundled")]
    {
        let mut hasher = Sha256::new();
        hasher.update(BUNDLE_BYTES);
        to_hex(&hasher.finalize())
    }
    #[cfg(not(feature = "bundled"))]
    {
        String::new()
    }
}

/// Internal helper function.
pub fn cache_root() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| Error::InvalidInput("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("bundled");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| Error::InvalidInput("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("bundled");
        Ok(path)
    }
}

/// Internal helper function.
#[cfg(feature = "bundled")]
fn extract_zip(dest: &Path) -> Result<()> {
    use std::io::{self, Cursor};

    let reader = Cursor::new(BUNDLE_BYTES);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| Error::InvalidInput(format!("zip open error: {e}")))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| Error::InvalidInput(format!("zip read error: {e}")))?;
        let name = file.name().to_string();
        let outpath = dest.join(&name);

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = File::create(&outpath)?;
        io::copy(&mut file, &mut outfile)?;
    }

    Ok(())
}

#[cfg(all(feature = "bundled", unix))]
/// Internal helper function.
fn ensure_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(feature = "bundled")]
/// Internal helper function.
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}
