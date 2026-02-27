# WinUI App

[中文](../../zh-CN/gui/winui-app.md)

## 1. Overview

The Windows app is built with WinUI 3 (.NET) and calls Rust FFI through `awmkit_native.dll`.

Its page semantics are aligned with macOS: embed, detect, database management, and key management.
The key page supports slot operations, random key generation, `.bin` import/export, and 64-char hex import (`0x` prefix allowed).

## 1.1 UI Text Contract

- Human-readable UI copy is a presentation-layer contract and may change between versions.
- Machine interfaces are unchanged (FFI structs and CLI JSON are still compatibility targets).
- Default UI only shows user-facing messages; internal diagnostics are behind a "Show diagnostics" switch.
- Canonical terms are defined in [`../ui/terminology.md`](../ui/terminology.md).

## 2. Development Build

```powershell
# 1) Build Rust FFI library
cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc

# 2) Build WinUI (project auto-prepares awmkit_native.dll and FFmpeg runtime)
cd winui-app/AWMKit
dotnet build -c Debug -p:Platform=x64
```

## 3. Release (local-first)

Recommended: use local script to produce an Inno installer (multi-file install layout, avoids single-file extraction path issues):

```powershell
powershell -File scripts/release/local-release-win.ps1
```

Manual publish (multi-file) if needed:

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj \
  -c Release -r win-x64 \
  -p:Platform=x64 \
  -p:SelfContained=true \
  -p:PublishSingleFile=false \
  -p:PublishTrimmed=false \
  -p:PublishAot=false
```

## 4. Runtime Dependencies

- Bundled mode expects `bundled/audiowmark-windows-x86_64.zip`
- Database path: `%LOCALAPPDATA%\\awmkit\\awmkit.db`
- Key/mapping/evidence operations are routed through Rust FFI
- Release layout includes: `AWMKit.exe`, `awmkit_native.dll`, `lib\\ffmpeg\\*.dll`, `bundled\\...`, `cli\\awmkit.exe`
