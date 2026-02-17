mod extract;
mod lock;
mod payload;
mod spawn;

use keyring::Entry;
use std::collections::HashSet;
use std::ffi::OsString;
#[cfg(windows)]
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "com.awmkit.watermark";
const SLOT_USERNAME_PREFIX: &str = "signing-key-slot-";
const LEGACY_USERNAME: &str = "signing-key";
const SLOT_MIN: u8 = 0;
const SLOT_MAX: u8 = 31;

enum LauncherCommand {
    CacheHelp,
    CacheClean { delete_db: bool, confirmed: bool },
    Forward(Vec<OsString>),
}

/// # Errors
/// 当运行时解包、核心进程拉起、参数解析或缓存清理失败时返回错误。
pub fn run() -> Result<i32, String> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse_command(&args)? {
        LauncherCommand::Forward(forward_args) => {
            let runtime = payload::prepare_runtime()?;
            spawn::run_core(&runtime, &forward_args)
        }
        LauncherCommand::CacheHelp => {
            eprintln!("usage: awmkit cache clean [--db] --yes");
            Ok(0)
        }
        LauncherCommand::CacheClean {
            delete_db,
            confirmed,
        } => run_cache_clean(delete_db, confirmed),
    }
}

fn parse_command(args: &[OsString]) -> Result<LauncherCommand, String> {
    let Some(first) = args.first() else {
        return Ok(LauncherCommand::Forward(Vec::new()));
    };
    if first != "cache" {
        return Ok(LauncherCommand::Forward(args.to_vec()));
    }

    if args.get(1).is_none() {
        return Ok(LauncherCommand::CacheHelp);
    }
    if args
        .get(1)
        .is_some_and(|value| value == "help" || value == "--help" || value == "-h")
    {
        return Ok(LauncherCommand::CacheHelp);
    }

    if args.get(1).is_none_or(|value| value != "clean") {
        return Err(
            "unknown cache subcommand; supported: awmkit cache clean [--db] --yes".to_string(),
        );
    }

    let mut delete_db = false;
    let mut confirmed = false;
    for arg in &args[2..] {
        match arg.to_string_lossy().as_ref() {
            "--db" => delete_db = true,
            "--yes" | "-y" => confirmed = true,
            "--help" | "-h" => {
                return Ok(LauncherCommand::CacheHelp);
            }
            unknown => {
                return Err(format!("unknown flag for cache clean: {unknown}"));
            }
        }
    }

    Ok(LauncherCommand::CacheClean {
        delete_db,
        confirmed,
    })
}

fn run_cache_clean(delete_db: bool, confirmed: bool) -> Result<i32, String> {
    if !confirmed {
        return Err("refusing to clean without confirmation; pass --yes".to_string());
    }

    let runtime_removed = payload::clear_runtime_root()?;
    eprintln!("runtime cleanup: {runtime_removed}");

    if delete_db {
        let db_removed = payload::clear_db_and_config()?;
        eprintln!("database/config cleanup: {db_removed}");
    }

    let key_slots = configured_key_slots();
    if key_slots > 0 {
        eprintln!(
            "note: detected {key_slots} configured key slot(s). keys are not deleted automatically."
        );
    }

    Ok(0)
}

fn configured_key_slots() -> usize {
    let mut slots = HashSet::new();
    for slot in SLOT_MIN..=SLOT_MAX {
        let username = format!("{SLOT_USERNAME_PREFIX}{slot}");
        if entry_has_password(&username) {
            slots.insert(slot);
        }
    }

    if entry_has_password(LEGACY_USERNAME) {
        slots.insert(0);
    }

    #[cfg(windows)]
    {
        let mut keys_dir = payload::app_base_dir().unwrap_or_else(|_| PathBuf::from("."));
        keys_dir.push("keys");
        if let Ok(entries) = std::fs::read_dir(keys_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if let Some(slot) = parse_dpapi_slot(&file_name) {
                    slots.insert(slot);
                }
            }
        }
    }

    slots.len()
}

fn entry_has_password(username: &str) -> bool {
    Entry::new(KEYRING_SERVICE, username)
        .and_then(|entry| entry.get_password())
        .is_ok()
}

#[cfg(windows)]
fn parse_dpapi_slot(file_name: &str) -> Option<u8> {
    let Some(raw) = file_name.strip_prefix("slot-") else {
        return None;
    };
    let Some(raw) = raw.strip_suffix(".dpapi") else {
        return None;
    };
    raw.parse::<u8>().ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_command, LauncherCommand};
    use std::ffi::OsString;

    #[test]
    fn parse_cache_clean_flags() {
        let args = vec![
            OsString::from("cache"),
            OsString::from("clean"),
            OsString::from("--db"),
            OsString::from("--yes"),
        ];
        let parsed = parse_command(&args).expect("parse should succeed");
        let LauncherCommand::CacheClean {
            delete_db,
            confirmed,
        } = parsed
        else {
            panic!("unexpected command");
        };
        assert!(delete_db);
        assert!(confirmed);
    }
}
