# 编译指南（当前）

本文件仅保留构建入口。详细构建矩阵与 CI 产物说明请查看：

- 中文：`docs/zh-CN/build/build-matrix.md`
- English: `docs/en-US/build/build-matrix.md`
- CI 产物（中文）：`docs/zh-CN/build/ci-artifacts.md`
- CI artifacts (English): `docs/en-US/build/ci-artifacts.md`

## 1. Rust CLI（推荐）

```bash
cargo build --bin awmkit --features full-cli --release
```

## 2. Rust FFI（GUI 依赖）

### macOS

```bash
cargo build --lib --features ffi,app,bundled --release
```

### Windows

```bash
cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc
```

## 3. Swift 绑定

```bash
cd bindings/swift && swift build
cd bindings/swift && swift test
```

## 4. macOS App

```bash
cd macos-app
xcodegen generate
xcodebuild -project AWMKit.xcodeproj -scheme AWMKit -configuration Debug -sdk macosx build
```

## 5. WinUI

```powershell
dotnet build winui-app/AWMKit/AWMKit.csproj -c Debug -p:Platform=x64
```

单文件发布参考：`docs/winui-publish-size-experiments.md`

## 6. 必要前置

- bundled 资源需可用：
  - `bundled/audiowmark-macos-arm64.zip`
  - `bundled/audiowmark-windows-x86_64.zip`
- `audiowmark` 可通过 bundled 自动解压；bundled 不可用时可用 `--audiowmark <PATH>` 回退。
