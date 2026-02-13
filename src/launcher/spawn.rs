use super::payload::PreparedRuntime;
use std::ffi::OsString;
use std::process::{Command, Stdio};

pub fn run_core(runtime: &PreparedRuntime, args: &[OsString]) -> Result<i32, String> {
    let mut command = Command::new(&runtime.core_binary);
    command.args(args);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    apply_runtime_env(&mut command, runtime)?;

    let status = command.status().map_err(|e| {
        format!(
            "failed to spawn core binary {}: {e}",
            runtime.core_binary.display()
        )
    })?;

    Ok(status.code().unwrap_or(1))
}

#[cfg(target_os = "windows")]
fn apply_runtime_env(command: &mut Command, runtime: &PreparedRuntime) -> Result<(), String> {
    let system_root =
        std::env::var_os("SystemRoot").unwrap_or_else(|| OsString::from(r"C:\Windows"));
    let mut path_value = OsString::new();
    path_value.push(runtime.runtime_dir.as_os_str());
    path_value.push(";");
    path_value.push(
        std::path::Path::new(&system_root)
            .join("System32")
            .as_os_str(),
    );
    path_value.push(";");
    path_value.push(system_root);
    command.env("PATH", path_value);
    command.env("AWMKIT_RUNTIME_STRICT", "1");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn apply_runtime_env(command: &mut Command, runtime: &PreparedRuntime) -> Result<(), String> {
    command.env("DYLD_LIBRARY_PATH", runtime.runtime_dir.as_os_str());
    command.env("AWMKIT_RUNTIME_STRICT", "1");
    Ok(())
}
