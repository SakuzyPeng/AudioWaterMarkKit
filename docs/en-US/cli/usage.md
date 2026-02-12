# CLI Usage

[中文](../../zh-CN/cli/usage.md)

## 1. Install and Build

### Use release binaries

- macOS: download `awmkit-macos-arm64.tar.gz`, then run `./awmkit`
- Windows: download `awmkit-windows-x86_64.zip`, then run `awmkit.exe`

### Build from source (recommended)

```bash
cargo build --bin awmkit --features full-cli --release
```

`full-cli` enables app-layer features, FFI, bundled resolution, and multichannel paths.

## 2. Supported Formats and Layouts

- Input audio: `wav` / `flac` / `m4a` / `alac`
- Output audio: `wav` (WAV-only output at the moment)
- Channel layout: `auto`, `stereo`, `surround51`, `surround512`, `surround71`, `surround714`, `surround916`

## 3. Global Options

```text
-v, --verbose
-q, --quiet
--audiowmark <PATH>
--lang <zh-CN|en-US>
```

## 4. Typical First-Run Flow

```bash
# 1) Initialize key in active slot
awmkit init

# 2) Encode (optional for debugging)
awmkit encode --tag SAKUZY

# 3) Embed (WAV-only output)
awmkit embed --tag SAKUZY input.wav --output output_wm.wav

# 4) Detect
awmkit detect output_wm.wav

# 5) System status
awmkit status --doctor
```

## 5. Command Overview

- `init`: initialize key in active slot
- `tag`: tag mapping helper commands
  - `suggest`, `save`, `list`, `remove`, `clear`
- `key`: key and slot management
  - `show`, `import`, `export`, `rotate`, `delete`
  - `slot current/use/list/label set/label clear`
- `encode` / `decode`: message codec utilities
- `embed`: watermark embedding (batch inputs supported)
- `detect`: detect and decode (`--json` supported)
- `evidence`: evidence query and cleanup
  - `list/show/remove/clear`
- `status`: system status and diagnostics

## 6. Key Slot Examples

```bash
# current active slot
awmkit key slot current

# switch active slot
awmkit key slot use 2

# rotate key in a specific slot
awmkit key rotate --slot 2

# delete key in slot (requires confirmation)
awmkit key delete --slot 2 --yes
```

## 7. Evidence Examples

```bash
# list latest 20 evidence rows
awmkit evidence list --limit 20

# filter by identity + slot
awmkit evidence list --identity SAKUZY --key-slot 0

# remove one evidence row
awmkit evidence remove 123 --yes

# filtered cleanup
awmkit evidence clear --identity SAKUZY --key-slot 0 --yes
```

Notes:
- Inputs that already contain watermarks are skipped automatically, with a batch summary warning at the end.
- `evidence list/show` and `evidence --json` do not surface the legacy forced marker field.

## 8. Detect JSON Fields

Common fields from `awmkit detect --json`:

- Core result: `status`, `tag`, `identity`, `version`, `key_slot`
- Slot diagnostics: `decode_slot_hint`, `decode_slot_used`, `slot_status`, `slot_scan_count`
- Evidence matching: `clone_check`, `clone_score`, `clone_match_seconds`, `clone_matched_evidence_id`

## 9. Exit Code Behavior

- Non-zero on runtime failure (invalid args, IO failures, invalid/error detect path).
- `clone_check=suspect` is a result annotation and does not independently force failure.
