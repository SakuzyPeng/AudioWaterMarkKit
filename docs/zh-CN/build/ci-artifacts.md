# CI 产物与发布

[English](../../en-US/build/ci-artifacts.md)

> 注：当前阶段发布以本地脚本为主（见 `/scripts/release/`），CI workflow 作为参考与备份流程。

## 主要 Workflow

- `.github/workflows/windows-winui.yml`
  - 构建 macOS App 包、WinUI 安装包与双平台 CLI 包
- `.github/workflows/build-audiowmark-macos-arm.yml`
  - 发布 `audiowmark-macos-arm64.tar.gz`
- `.github/workflows/build-audiowmark-windows-release.yml`
  - 发布 `audiowmark-windows-x86_64.zip`
- `.github/workflows/build-ffmpeg-runtime.yml`
  - 构建并发布 FFmpeg 运行时资产
- `.github/workflows/check-ffi-bindings.yml`
  - 校验 FFI 头文件同步与 Swift Package 可编译

## 已废弃并移除的 Workflow

- `.github/workflows/build-awmkit.yml`
  - 旧版独立 CLI 发布流程，已并入 `windows-winui.yml` / 本地发布脚本。
- `.github/workflows/build-audiowmark-windows.yml`
  - 旧版 Windows audiowmark 构建流程，已由 `build-audiowmark-windows-release.yml` 替代。
- `.github/workflows/build-windows-deps.yml`
  - 旧版依赖预构建流程，当前发布链路不再使用。
- `.github/workflows/build-audiowmark-windows.yml.cygwin.bak`
  - 历史备份文件，避免与有效 workflow 混淆已移除。

## CLI 发布产物

当前 CLI 产物由 `windows-winui.yml`（CI）与本地发布脚本统一产出，
均为 launcher 单文件（`awmkit` / `awmkit.exe`），
运行时依赖会在首次运行时解压到用户目录 runtime。

## WinUI / macOS App 产物

`windows-winui.yml` 上传 artifact：

- `dist/macos/AWMKit-macos-arm64.app.zip`
- `dist/macos/awmkit-macos-arm64-cli-single-file.zip`
- `dist/local/AWMKit-win-x64-ui-installer-*.exe`
- `dist/windows/awmkit-windows-x64-cli-single-file.zip`

## bundled 依赖来源

CI 会从 GitHub Release 下载并准备：

- `audiowmark-macos-arm64.tar.gz`
- `audiowmark-windows-x86_64.zip`

对应运行时 zip：

- `bundled/audiowmark-macos-arm64.zip`
- `bundled/audiowmark-windows-x86_64.zip`

## 发布前建议

1. 先验证 `cargo test --features app`。
2. 校验 CLI 帮助与文档一致（`awmkit --help`）。
3. 确认 Release Notes：`docs/AWMKIT_RELEASE_NOTES.md`。
4. 本地发布优先执行：
   - `./scripts/release/local-release-macos.sh`
   - `powershell -File scripts/release/local-release-win.ps1`（在 `win-pc`）
