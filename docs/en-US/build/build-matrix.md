# Build Matrix

[中文](../../zh-CN/build/build-matrix.md)

## Rust Targets and Commands

| Target | Command | Notes |
| --- | --- | --- |
| Core library (multichannel by default) | `cargo build --release` | Rust library only |
| CLI (recommended for release) | `cargo build --bin awmkit --features full-cli --release` | Self-contained runtime strategy (bundled-first, FFmpeg decode enabled) |
| FFI (macOS app) | `cargo build --lib --features ffi,app,bundled --release` | Used by Swift/GUI (FFmpeg decode enabled) |
| FFI (Windows) | `cargo build --lib --features ffi,app,bundled --release --target x86_64-pc-windows-msvc` | Used by WinUI (FFmpeg decode enabled) |

## Validation Commands

```bash
cargo test --features app
cargo clippy --all-features
```

## GUI Build Entrypoints

- macOS: `xcodegen generate` + `xcodebuild`
- WinUI: `dotnet build -p:Platform=x64`

## Feature Notes (current)

- Default feature: `multichannel`
- `app`: config, i18n, database, evidence, key management
- `bundled`: bundled audiowmark extraction and bundled-first resolution
- `ffmpeg-decode`: FFmpeg dynamic-library decode backend
- `full-cli`: release-grade CLI bundle (`app + bundled + ffi + ffmpeg-decode` + CLI deps)

## Build Outputs

- CLI: `target/<target>/release/awmkit(.exe)`
- macOS FFI: `target/release/libawmkit.dylib`
- Windows FFI: `target/x86_64-pc-windows-msvc/release/awmkit.dll`
