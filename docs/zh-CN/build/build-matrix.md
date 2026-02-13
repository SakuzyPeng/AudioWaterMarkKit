# 构建矩阵

[English](../../en-US/build/build-matrix.md)

## Rust 目标与命令

| 目标 | 命令 | 说明 |
| --- | --- | --- |
| 核心库（默认含 multichannel） | `cargo build --release` | 仅 Rust 库 |
| CLI core | `cargo build --bin awmkit-core --features full-cli --release` | 真实 CLI 实现（bundled 优先，FFmpeg 解码） |
| CLI launcher（单文件入口） | `cargo build --bin awmkit --features launcher --release` | 负责解压运行时并转发参数到 core |
| FFI（macOS App） | `cargo build --lib --features ffi,app,bundled --release` | 供 Swift/GUI 调用（含 FFmpeg 解码） |
| FFI（Windows） | `cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc` | 供 WinUI 调用（含 FFmpeg 解码） |

## 验证命令

```bash
cargo test --features app
cargo clippy --all-features
```

## GUI 构建入口

- macOS：`xcodegen generate` + `xcodebuild`
- WinUI：`dotnet build -p:Platform=x64`

## Feature 说明（当前）

- 默认 feature：`multichannel`
- `app`：配置、i18n、数据库、证据、密钥管理
- `bundled`：启用 bundled audiowmark 解压与优先解析
- `ffmpeg-decode`：启用 FFmpeg 动态库解码后端
- `full-cli`：CLI 发布组合（`app + bundled + ffi + ffmpeg-decode` 与 CLI 依赖）

## 产物说明

- CLI launcher：`target/<target>/release/awmkit(.exe)`
- CLI core：`target/<target>/release/awmkit-core(.exe)`
- macOS FFI：`target/release/libawmkit.dylib`
- Windows FFI：`target/x86_64-pc-windows-msvc/release/awmkit.dll`
