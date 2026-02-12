# WinUI 应用

[English](../../en-US/gui/winui-app.md)

## 1. 概览

Windows 端使用 WinUI 3（.NET），并通过 `awmkit_native.dll` 调用 Rust FFI。

页面结构与 macOS 语义对齐：嵌入、检测、数据库管理、密钥管理。

## 2. 开发构建

```powershell
# 1) Rust 侧构建 FFI
cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc

# 2) 构建 WinUI（项目会自动准备 awmkit_native.dll 与 FFmpeg runtime）
cd winui-app/AWMKit
dotnet build -c Debug -p:Platform=x64
```

## 3. 发布（本地优先）

当前推荐使用本地脚本生成 Inno 安装包（多文件安装目录，避免单文件自解压路径问题）：

```powershell
powershell -File scripts/release/local-release-win.ps1
```

如需手动发布（非单文件）：

```powershell
dotnet publish winui-app/AWMKit/AWMKit.csproj \
  -c Release -r win-x64 \
  -p:Platform=x64 \
  -p:SelfContained=true \
  -p:PublishSingleFile=false \
  -p:PublishTrimmed=false \
  -p:PublishAot=false
```

## 4. 运行依赖

- bundled 模式依赖 `bundled/audiowmark-windows-x86_64.zip`
- 数据库路径：`%LOCALAPPDATA%\\awmkit\\awmkit.db`
- 密钥、映射、证据操作已统一走 Rust FFI
- 发布目录包含：`AWMKit.exe`、`awmkit_native.dll`、`lib\\ffmpeg\\*.dll`、`bundled\\...`、`cli\\awmkit.exe`
