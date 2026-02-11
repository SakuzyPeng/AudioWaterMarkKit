# Win+mac 开发同步工作流（Git 主链 + cwRsync 应急）

## 目标
- 日常开发以 `Git` 为唯一真源，避免跨机代码漂移。
- `win-pc` 默认只执行：`pull -> build/test -> publish`。
- `cwRsync` 仅用于应急，不用于发布前最终状态。

## 默认流程（Git-first）
1. 在 mac 主工作区完成修改、提交、推送：
   - `git push origin master`
2. 在 mac 触发远端同步脚本（推荐）：
   - `scripts/sync_win_git.sh --host win-pc --build all`
3. 在 `win-pc` 验证结果：
   - Rust FFI 构建
   - WinUI 构建
   - （可选）单文件发布

## 脏工作区策略（强制）
- `win-pc` 拉取前会检查工作区是否 clean。
- 若 dirty，脚本立即失败并输出改动清单。
- 不自动 `stash`，不自动 `hard reset`。

处理建议：
1. 先在 `win-pc` 手动提交或丢弃本地临时改动。
2. 确认 `git status --short` 为空。
3. 重新执行 `scripts/sync_win_git.sh`。

## cwRsync 应急流程（仅应急）
- 适用场景：
  - 网络异常导致 pull 慢/失败
  - 临时验证不可提交草稿
- 执行：
  - `scripts/sync_win_rsync_emergency.sh --host win-pc --emergency`
- 应急结束必须回归：
  1. 在 mac 提交并推送 Git 真正版本
  2. 在 win-pc 执行 Git 主流程同步

## 风险说明（cwRsync）
- 可能覆盖 win-pc 未提交改动。
- 可能出现“本地可跑但 Git 不可复现”的状态。
- 因此仅建议短期试验，不用于交付版本。

## 构建命令矩阵（win-pc）
- Rust FFI（Windows target）：
  - `cargo build --lib --features ffi,app,bundled,multichannel --release --target x86_64-pc-windows-msvc`
- WinUI Debug：
  - `dotnet build winui-app/AWMKit/AWMKit.csproj -c Debug -p:Platform=x64`
- WinUI 单文件发布：
  - `dotnet publish winui-app/AWMKit/AWMKit.csproj -c Release -r win-x64 -p:Platform=x64 --self-contained`

## 常用命令
- 仅同步（不构建）：
  - `scripts/sync_win_git.sh --host win-pc --build none`
- 同步并完整构建：
  - `scripts/sync_win_git.sh --host win-pc --build all`
- 同步并仅 WinUI 构建：
  - `scripts/sync_win_git.sh --host win-pc --build winui`

