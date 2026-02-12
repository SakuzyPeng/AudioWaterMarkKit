# Native macOS App

[中文](../../zh-CN/gui/macos-app.md)

## 1. Overview

The macOS app is built with SwiftUI and uses Rust FFI as the single source of core behavior:

- Embed page (input source, settings, queue, logs)
- Detect page (result card, queue, logs, selection linkage)
- Tag/Database page (mapping + evidence management)
- Key page (slots, generate/delete, labels, summaries)

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
- Successful embed writes evidence; detect clone-check can match against evidence rows
