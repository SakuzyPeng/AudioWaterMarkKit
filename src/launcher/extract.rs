use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

pub fn extract_zip_payload(payload_zip: &[u8], destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("destination has no parent: {}", destination.display()))?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_nanos();
    let temp_dir = parent.join(format!(".tmp-{}-{nanos}", std::process::id()));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("failed to clean stale temp dir {}: {e}", temp_dir.display()))?;
    }
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("failed to create temp dir {}: {e}", temp_dir.display()))?;

    let extract_result = extract_all(payload_zip, &temp_dir);
    if let Err(err) = extract_result {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(err);
    }

    fs::rename(&temp_dir, destination).map_err(|e| {
        let _ = fs::remove_dir_all(&temp_dir);
        format!(
            "failed to move extracted payload {} -> {}: {e}",
            temp_dir.display(),
            destination.display()
        )
    })?;

    Ok(())
}

fn extract_all(payload_zip: &[u8], output_dir: &Path) -> Result<(), String> {
    let cursor = Cursor::new(payload_zip);
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("invalid payload zip: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read payload zip entry #{i}: {e}"))?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| format!("payload zip entry contains unsafe path: {}", entry.name()))?;
        let out_path: PathBuf = output_dir.join(enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("failed to create directory {}: {e}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory {}: {e}", parent.display()))?;
        }
        let mut out_file = File::create(&out_path)
            .map_err(|e| format!("failed to create {}: {e}", out_path.display()))?;
        io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(mode);
            fs::set_permissions(&out_path, perms)
                .map_err(|e| format!("failed to set permissions on {}: {e}", out_path.display()))?;
        }
    }

    Ok(())
}
