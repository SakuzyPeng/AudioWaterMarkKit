# WinUI App

[中文](../../zh-CN/gui/winui-app.md)

## 1. Overview

The Windows app is built with WinUI 3 (.NET) and calls Rust FFI through `awmkit_native.dll`.

Its page semantics are aligned with macOS: embed, detect, database management, and key management.

## 2. Development Build

```powershell
# 1) Build Rust FFI library
cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc

# 2) Copy dll (before local WinUI debug)
Copy-Item target/x86_64-pc-windows-msvc/release/awmkit.dll winui-app/AWMKit/awmkit_native.dll -Force

# 3) Build WinUI
cd winui-app/AWMKit
dotnet build -c Debug -p:Platform=x64
```

## 3. Single-file Publish (recommended)

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj \
  -c Release -r win-x64 \
  -p:Platform=x64 \
  -p:PublishSingleFile=true \
  -p:SelfContained=true \
  -p:IncludeNativeLibrariesForSelfExtract=true \
  -p:IncludeAllContentForSelfExtract=true \
  -p:PublishTrimmed=true \
  -p:TrimMode=partial \
  -p:EnableCompressionInSingleFile=true \
  -p:PublishAot=false
```

See size benchmark notes: [`docs/winui-publish-size-experiments.md`](../../winui-publish-size-experiments.md)

## 4. Runtime Dependencies

- Bundled mode expects `bundled/audiowmark-windows-x86_64.zip`
- Database path: `%LOCALAPPDATA%\\awmkit\\awmkit.db`
- Key/mapping/evidence operations are routed through Rust FFI
