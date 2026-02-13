# CI Artifacts and Release

[中文](../../zh-CN/build/ci-artifacts.md)

> Note: at the current stage, release is driven by local scripts (see `/scripts/release/`). CI workflows are maintained as reference/backup.

## Primary Workflows

- `.github/workflows/windows-winui.yml`
  - Builds macOS app package, WinUI installer, and cross-platform CLI packages
- `.github/workflows/build-audiowmark-macos-arm.yml`
  - Publishes `audiowmark-macos-arm64.tar.gz`
- `.github/workflows/build-audiowmark-windows-release.yml`
  - Publishes `audiowmark-windows-x86_64.zip`
- `.github/workflows/build-ffmpeg-runtime.yml`
  - Builds and publishes FFmpeg runtime assets
- `.github/workflows/check-ffi-bindings.yml`
  - Verifies FFI header sync and Swift Package buildability

## Deprecated And Removed Workflows

- `.github/workflows/build-awmkit.yml`
  - Legacy standalone CLI release flow, superseded by `windows-winui.yml` and local release scripts.
- `.github/workflows/build-audiowmark-windows.yml`
  - Legacy Windows audiowmark build flow, replaced by `build-audiowmark-windows-release.yml`.
- `.github/workflows/build-windows-deps.yml`
  - Legacy dependency prebuild flow, no longer used in the current release pipeline.
- `.github/workflows/build-audiowmark-windows.yml.cygwin.bak`
  - Historical backup file; removed to avoid confusion with active workflows.

## CLI Release Artifacts

CLI artifacts are now produced by `windows-winui.yml` (CI) and local release scripts.
Both archives carry a single launcher binary (`awmkit` / `awmkit.exe`), and runtime dependencies
are extracted on first run into user-local runtime directories.

## WinUI / macOS App Artifacts

Artifacts uploaded by `windows-winui.yml`:

- `dist/macos/AWMKit-macos-arm64.app.zip`
- `dist/macos/awmkit-macos-arm64-cli-single-file.zip`
- `dist/local/AWMKit-win-x64-ui-installer-*.exe`
- `dist/windows/awmkit-windows-x64-cli-single-file.zip`

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
4. Prefer local release scripts first:
   - `./scripts/release/local-release-macos.sh`
   - `powershell -File scripts/release/local-release-win.ps1` (on `win-pc`)
