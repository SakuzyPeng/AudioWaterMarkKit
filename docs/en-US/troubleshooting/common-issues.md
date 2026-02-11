# Common Issues

[中文](../../zh-CN/troubleshooting/common-issues.md)

## 1. `audiowmark` not found

Symptom: `awmkit status --doctor` reports engine unavailable.  
Check in this order:

1. Ensure bundled zip exists (`bundled/audiowmark-macos-arm64.zip` / `bundled/audiowmark-windows-x86_64.zip`).
2. Ensure cache directory is writable (`~/.awmkit/bundled` or `%LOCALAPPDATA%\\awmkit\\bundled`).
3. Use fallback path temporarily: `--audiowmark <PATH>`.

## 2. WinUI `EntryPointNotFound`

Typical cause: `awmkit_native.dll` does not match current source build.  
Fix:

1. Rebuild Rust FFI: `cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc`
2. Replace dll: `awmkit.dll -> winui-app/AWMKit/awmkit_native.dll`
3. Restart app.

## 3. Cannot embed/detect on first run

Cause: no key is configured yet.  
Fix: generate key in Key page, or run `awmkit init` in CLI.

## 4. `invalid_hmac` detect result

Meaning: a candidate message was found, but HMAC verification failed with available keys.  
Common causes:

- wrong slot or wrong key
- key replaced while validating old samples

Use `detect --json` diagnostics: `decode_slot_hint`, `decode_slot_used`, `slot_status`.

## 5. Database status unavailable (red)

Check:

1. Database path is accessible.
2. File is not locked by another process.
3. Backup first, then recover/recreate DB if needed (`~/.awmkit/awmkit.db` / `%LOCALAPPDATA%\\awmkit\\awmkit.db`).
