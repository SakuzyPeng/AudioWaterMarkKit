# CI Artifacts and Release

[中文](../../zh-CN/build/ci-artifacts.md)

## Primary Workflows

- `.github/workflows/build-awmkit.yml`
  - Builds and releases CLI (macOS arm64 + Windows x64)
- `.github/workflows/windows-winui.yml`
  - Builds macOS app package and WinUI single-file artifact
- `.github/workflows/build-audiowmark-macos-arm.yml`
  - Publishes `audiowmark-macos-arm64.tar.gz`
- `.github/workflows/build-audiowmark-windows-release.yml`
  - Publishes `audiowmark-windows-x86_64.zip`

## CLI Release Artifacts

Artifacts from `build-awmkit.yml`:

- `awmkit-macos-arm64.tar.gz`
- `awmkit-windows-x86_64.zip`

Triggers:

- Tag push: `awmkit-*`
- Manual `workflow_dispatch`

## WinUI / macOS App Artifacts

Artifacts uploaded by `windows-winui.yml`:

- `dist/macos/AWMKit-macos-arm64.app.zip`
- `dist/macos/awmkit-macos-arm64`
- `dist/windows/AWMKit-win-x64-single.exe`
- `target/x86_64-pc-windows-msvc/release/awmkit.exe`
- `target/x86_64-pc-windows-msvc/release/awmkit.dll`

## Bundled Dependency Source

CI downloads and repacks releases for runtime use:

- `audiowmark-macos-arm64.tar.gz`
- `audiowmark-windows-x86_64.zip`

Runtime bundled zips:

- `bundled/audiowmark-macos-arm64.zip`
- `bundled/audiowmark-windows-x86_64.zip`

## Pre-release Checklist

1. Validate `cargo test --features app`.
2. Ensure CLI help matches docs (`awmkit --help`).
3. Update release notes at `docs/AWMKIT_RELEASE_NOTES.md`.
