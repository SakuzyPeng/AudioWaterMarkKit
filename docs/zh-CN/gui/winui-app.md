# WinUI 应用

[English](../../en-US/gui/winui-app.md)

## 1. 概览

Windows 端使用 WinUI 3（.NET），并通过 `awmkit_native.dll` 调用 Rust FFI。

页面结构与 macOS 语义对齐：嵌入、检测、数据库管理、密钥管理。

## 2. 开发构建

```powershell
# 1) Rust 侧构建 FFI
cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc

# 2) 复制 dll（调试本地 WinUI 前）
Copy-Item target/x86_64-pc-windows-msvc/release/awmkit.dll winui-app/AWMKit/awmkit_native.dll -Force

# 3) 构建 WinUI
cd winui-app/AWMKit
dotnet build -c Debug -p:Platform=x64
```

## 3. 单文件发布（推荐参数）

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

详细尺寸实验见：[`docs/winui-publish-size-experiments.md`](../../winui-publish-size-experiments.md)

## 4. 运行依赖

- bundled 模式依赖 `bundled/audiowmark-windows-x86_64.zip`
- 数据库路径：`%LOCALAPPDATA%\\awmkit\\awmkit.db`
- 密钥、映射、证据操作已统一走 Rust FFI
