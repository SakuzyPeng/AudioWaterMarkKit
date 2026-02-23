# Native macOS App

[中文](../../zh-CN/gui/macos-app.md)

## 1. Overview

The macOS app is built with SwiftUI and uses Rust FFI as the single source of core behavior:

- Embed page (input source, settings, queue, logs)
- Detect page (result card, queue, logs, selection linkage)
- Tag/Database page (mapping + evidence management)
- Key page (slots, generate/delete, import/export, hex import, labels, summaries)

## 2. Development Build

```bash
# 1) Build Rust dynamic library
cargo build --lib --features ffi,app,bundled --release

# 2) Generate Xcode project
cd macos-app
xcodegen generate

# 3) Build app
xcodebuild \
  -project AWMKit.xcodeproj \
  -scheme AWMKit \
  -configuration Debug \
  -sdk macosx \
  build
```

## 3. Runtime Dependencies

- Bundled mode requires `bundled/audiowmark-macos-arm64.zip`
- Database path: `~/.awmkit/awmkit.db`
- Key and slot operations are centralized in Rust (UI via FFI only)
- Pre-release/local builds are usually unsigned; if macOS reports “damaged”, run:
  - `xattr -dr com.apple.quarantine /path/to/AWMKit.app`

## 4. Common Validation Points

- Without key, embed is disabled; detect is still allowed with unverified-result warning
- Switching slot refreshes status tooltip immediately
- Key import supports 32-byte `.bin` and 64-char hex (`0x` prefix allowed); overwrite is blocked when slot already has key
- Successful embed writes evidence; detect clone-check can match against evidence rows

## 5. Keychain Authorization Notes (macOS only)

- macOS Keychain grants access per entry. AWMKit currently stores one key entry per slot, so first access to multiple configured slots can trigger multiple prompts.
- Choosing `Always Allow` usually prevents repeated prompts for the same app identity + entry.
- CLI and App share the same key backend, so both are affected by the same Keychain authorization policy.
- If app identity changes (for example unsigned/ad-hoc signed builds, reinstall, or signing changes after update), macOS may treat it as a new app and request authorization again.
- This is expected system security behavior, not a KeyStore logic defect in AWMKit.
