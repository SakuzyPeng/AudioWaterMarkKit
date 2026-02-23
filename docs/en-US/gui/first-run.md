# UI First-Run Guide

[中文](../../zh-CN/gui/first-run.md)

This guide is for first-time users of AWMKit GUI (macOS App and WinUI App).

## 1. Pre-flight Checklist

1. Launch the app and confirm top-right status icons are available (key/engine/database).
2. Prepare test audio files (start with `wav` or `flac`).
3. Current embed output is `WAV` only (`FLAC` output is temporarily disabled).

## 2. First Action After Launch

1. Open the `Key` page.
2. Select a slot (default is `0`).
3. Configure key material using one of these methods:
   - Click `Generate` to create a random key.
   - Click `Import (.bin)` to import a 32-byte key file.
   - Click `Hex Import` and paste a 64-character hex key (`0x` prefix allowed).
4. Optional: click `Export (.bin)` to back up the selected slot key.
5. Optional: set a label for the active slot.

Notes:
- Without a key, `Embed` is disabled.
- `Detect` is still allowed, but results may be unverified (reference only, not for attribution/forensics).
- macOS only: keys are currently stored as separate Keychain entries per slot, so first access to multiple configured slots may trigger multiple prompts.
- macOS only: choosing `Always Allow` usually suppresses repeated prompts for the same app identity + entry.
- macOS only: if app identity changes (for example unsigned/ad-hoc signed builds, reinstall, or signing changes after update), Keychain may ask again. This is expected system behavior, not an AWMKit functional failure.

## 3. Create a Tag Mapping (Recommended)

1. Open the `Tags` page (database page).
2. Click `Add Mapping` and input a username.
3. Save to generate and persist the corresponding tag mapping.

Later, embedding with that username reuses the stored mapping automatically.

## 4. First Embed Run

1. Open the `Embed` page.
2. Select input file or directory from the input summary card (directory scan is current-level only).
3. Optional: set output directory (if empty, output goes back to source folder).
4. Enter username and confirm tag preview.
5. Click `Embed`.

Key behaviors:
- Drag-and-drop supports both files and directories; drag only affects queue and does not change input source address.
- Inputs that already contain watermarks are auto-skipped with a batch summary warning at the end.
- Success logs include SNR when available.

## 5. First Detect Run

1. Open the `Detect` page.
2. Select/drag files or directories.
3. Click `Detect`.
4. Check the result card for `status/tag/identity/key_slot` and related fields.

Notes:
- Without key or with failed verification, parsed fields are still shown and marked with unverified warning.
- Unverified results must not be used for attribution/forensics.

## 6. Review Evidence

1. Go back to `Tags` (database page).
2. Review audio evidence on the right panel (identity, tag, slot, path, time, etc.).
3. Use search to filter by username/tag/identity/path/SHA256.

## 7. Common Data Paths

1. Database:
   - macOS / Linux: `~/.awmkit/awmkit.db`
   - Windows: `%LOCALAPPDATA%\\awmkit\\awmkit.db`
2. Bundled audiowmark cache:
   - macOS: `~/.awmkit/bundled/bin/audiowmark`
   - Windows: `%LOCALAPPDATA%\\awmkit\\bundled\\bin\\audiowmark.exe`

## 8. Next Steps

1. CLI details: [`../cli/usage.md`](../cli/usage.md)
2. Troubleshooting: [`../troubleshooting/common-issues.md`](../troubleshooting/common-issues.md)
