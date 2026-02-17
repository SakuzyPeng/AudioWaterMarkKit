use super::extract::extract_zip_payload;
use super::lock::ExtractLock;
use serde::Deserialize;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/launcher_payload.rs"));

const READY_MARKER: &str = ".ready";
const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug)]
pub struct PreparedRuntime {
    pub runtime_dir: PathBuf,
    pub core_binary: PathBuf,
}

#[derive(Debug, Deserialize)]
struct PayloadManifest {
    core_binary: String,
}

pub fn prepare_runtime() -> Result<PreparedRuntime, String> {
    if PAYLOAD.is_empty() {
        return prepare_dev_runtime();
    }
    if PAYLOAD_SHA256.is_empty() {
        return Err("embedded launcher payload hash is empty".to_string());
    }

    let runtime_root = runtime_root()?;
    fs::create_dir_all(&runtime_root).map_err(|e| {
        format!(
            "failed to create runtime root {}: {e}",
            runtime_root.display()
        )
    })?;

    let runtime_dir = runtime_root.join(PAYLOAD_SHA256);
    let ready_marker = runtime_dir.join(READY_MARKER);
    if ready_marker.is_file() {
        let runtime = load_prepared_runtime(&runtime_dir)?;
        cleanup_old_runtime_dirs(&runtime_root, PAYLOAD_SHA256)?;
        return Ok(runtime);
    }

    let lock_path = runtime_root.join(".extract.lock");
    let _lock = ExtractLock::acquire(&lock_path)?;

    if ready_marker.is_file() {
        let runtime = load_prepared_runtime(&runtime_dir)?;
        cleanup_old_runtime_dirs(&runtime_root, PAYLOAD_SHA256)?;
        return Ok(runtime);
    }

    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir).map_err(|e| {
            format!(
                "failed to remove stale runtime dir {}: {e}",
                runtime_dir.display()
            )
        })?;
    }

    extract_zip_payload(PAYLOAD, &runtime_dir)?;
    fs::write(&ready_marker, PAYLOAD_SHA256.as_bytes()).map_err(|e| {
        format!(
            "failed to write runtime marker {}: {e}",
            ready_marker.display()
        )
    })?;

    let runtime = load_prepared_runtime(&runtime_dir)?;
    cleanup_old_runtime_dirs(&runtime_root, PAYLOAD_SHA256)?;
    Ok(runtime)
}

pub fn app_base_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| "LOCALAPPDATA/APPDATA not set".to_string())?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME not set".to_string())?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        Ok(path)
    }
}

pub fn runtime_root() -> Result<PathBuf, String> {
    let mut path = app_base_dir()?;
    path.push("runtime");
    Ok(path)
}

pub fn db_path() -> Result<PathBuf, String> {
    let mut path = app_base_dir()?;
    path.push("awmkit.db");
    Ok(path)
}

pub fn config_path() -> Result<PathBuf, String> {
    let mut path = app_base_dir()?;
    path.push("config.toml");
    Ok(path)
}

pub fn clear_runtime_root() -> Result<&'static str, String> {
    let path = runtime_root()?;
    if !path.exists() {
        return Ok("already empty");
    }
    fs::remove_dir_all(&path)
        .map_err(|e| format!("failed to remove runtime root {}: {e}", path.display()))?;
    Ok("removed")
}

pub fn clear_db_and_config() -> Result<&'static str, String> {
    let db_path = db_path()?;
    if db_path.exists() {
        fs::remove_file(&db_path)
            .map_err(|e| format!("failed to remove db {}: {e}", db_path.display()))?;
    }

    let config_path = config_path()?;
    if config_path.exists() {
        fs::remove_file(&config_path)
            .map_err(|e| format!("failed to remove config {}: {e}", config_path.display()))?;
    }

    Ok("removed")
}

fn prepare_dev_runtime() -> Result<PreparedRuntime, String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("failed to resolve launcher path: {e}"))?;
    let Some(dir) = exe_path.parent() else {
        return Err("failed to resolve launcher directory".to_string());
    };
    let core_name = core_binary_name();
    let core_path = dir.join(core_name);
    if !core_path.is_file() {
        return Err(
            "launcher payload is not embedded and sibling awmkit-core is missing".to_string(),
        );
    }
    Ok(PreparedRuntime {
        runtime_dir: dir.to_path_buf(),
        core_binary: core_path,
    })
}

fn load_prepared_runtime(runtime_dir: &Path) -> Result<PreparedRuntime, String> {
    let manifest = read_manifest(runtime_dir)?;
    let core_path = runtime_dir.join(&manifest.core_binary);
    if !core_path.is_file() {
        return Err(format!(
            "runtime core binary missing: {}",
            core_path.display()
        ));
    }
    #[cfg(unix)]
    ensure_executable(&core_path)?;
    Ok(PreparedRuntime {
        runtime_dir: runtime_dir.to_path_buf(),
        core_binary: core_path,
    })
}

fn read_manifest(runtime_dir: &Path) -> Result<PayloadManifest, String> {
    let manifest_path = runtime_dir.join(MANIFEST_FILE);
    let data = fs::read(&manifest_path).map_err(|e| {
        format!(
            "failed to read payload manifest {}: {e}",
            manifest_path.display()
        )
    })?;
    let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&data);
    serde_json::from_slice::<PayloadManifest>(data).map_err(|e| {
        format!(
            "failed to parse payload manifest {}: {e}",
            manifest_path.display()
        )
    })
}

fn cleanup_old_runtime_dirs(runtime_root: &Path, keep_hash: &str) -> Result<(), String> {
    let entries = fs::read_dir(runtime_root).map_err(|e| {
        format!(
            "failed to list runtime root {}: {e}",
            runtime_root.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read runtime entry: {e}"))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to inspect runtime entry: {e}"))?;
        if !file_type.is_dir() {
            continue;
        }
        let name_os: OsString = entry.file_name();
        let name = name_os.to_string_lossy();
        if name == keep_hash || name.starts_with(".tmp-") {
            continue;
        }
        fs::remove_dir_all(entry.path()).map_err(|e| {
            format!(
                "failed to remove stale runtime dir {}: {e}",
                entry.path().display()
            )
        })?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn core_binary_name() -> &'static str {
    "awmkit-core.exe"
}

#[cfg(not(target_os = "windows"))]
const fn core_binary_name() -> &'static str {
    "awmkit-core"
}

#[cfg(unix)]
fn ensure_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))?
        .permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms).map_err(|e| {
        format!(
            "failed to set executable permission {}: {e}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::PayloadManifest;

    #[test]
    fn manifest_with_utf8_bom_is_accepted() {
        let raw = b"\xEF\xBB\xBF{\"core_binary\":\"awmkit-core.exe\"}";
        let stripped = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
        let parsed: Result<PayloadManifest, _> = serde_json::from_slice(stripped);
        assert!(
            parsed.is_ok(),
            "manifest parse failed: {}",
            parsed.err().map(|e| e.to_string()).unwrap_or_default()
        );
        if let Ok(manifest) = parsed {
            assert_eq!(manifest.core_binary, "awmkit-core.exe");
        }
    }
}
