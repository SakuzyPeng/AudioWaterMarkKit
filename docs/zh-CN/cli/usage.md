# CLI 使用指南

[English](../../en-US/cli/usage.md)

## 1. 安装与构建

### 从 Release 使用

- macOS：下载 `awmkit-macos-arm64.tar.gz`，解压后运行 `./awmkit`
- Windows：下载 `awmkit-windows-x86_64.zip`，解压后运行 `awmkit.exe`

### 从源码构建（推荐命令）

```bash
cargo build --bin awmkit --features full-cli --release
```

`full-cli` 会启用应用层、FFI、bundled 运行逻辑和多声道路径。

## 2. 支持格式与布局

- 输入音频：`wav` / `flac` / `m4a` / `alac`
- 输出音频：`wav`（当前仅支持 WAV 输出）
- 声道布局：`auto`、`stereo`、`surround51`、`surround512`、`surround71`、`surround714`、`surround916`

## 3. 全局参数

```text
-v, --verbose
-q, --quiet
--audiowmark <PATH>
--lang <zh-CN|en-US>
```

## 4. 常用流程（首次）

```bash
# 1) 初始化密钥（当前激活槽位）
awmkit init

# 2) 编码（可选，便于调试）
awmkit encode --tag SAKUZY

# 3) 嵌入（当前仅输出 wav）
awmkit embed --tag SAKUZY input.wav --output output_wm.wav

# 风险场景下强制嵌入（证据会标记为强行嵌入）
awmkit embed --tag SAKUZY input.wav --output output_wm.wav --force-embed

# 4) 检测
awmkit detect output_wm.wav

# 5) 查看状态
awmkit status --doctor
```

## 5. 命令总览

- `init`：初始化当前激活槽位密钥
- `tag`：标签映射管理
  - `suggest`、`save`、`list`、`remove`、`clear`
- `key`：密钥与槽位管理
  - `show`、`import`、`export`、`rotate`、`delete`
  - `slot current/use/list/label set/label clear`
- `encode` / `decode`：消息编解码
- `embed`：嵌入水印（支持批量输入）
- `detect`：检测与解码（支持 `--json`）
- `evidence`：证据查询与删除
  - `list/show/remove/clear`
- `status`：系统状态与诊断

## 6. 密钥槽位示例

```bash
# 查看当前激活槽位
awmkit key slot current

# 切换激活槽位
awmkit key slot use 2

# 在指定槽位轮换密钥
awmkit key rotate --slot 2

# 删除槽位密钥（需要确认）
awmkit key delete --slot 2 --yes
```

## 7. 证据管理示例

```bash
# 查看最近 20 条证据
awmkit evidence list --limit 20

# 按 identity + 槽位过滤
awmkit evidence list --identity SAKUZY --key-slot 0

# 删除单条证据
awmkit evidence remove 123 --yes

# 条件清理
awmkit evidence clear --identity SAKUZY --key-slot 0 --yes
```

说明：
- `evidence list/show` 仅在强行嵌入记录上显示 `FORCED` / `is_forced_embed=true`。
- `evidence --json` 始终包含 `is_forced_embed` 布尔字段。

## 8. 检测 JSON 关键字段

`awmkit detect --json` 输出中常见字段：

- 结果字段：`status`、`tag`、`identity`、`version`、`key_slot`
- 解码槽位诊断：`decode_slot_hint`、`decode_slot_used`、`slot_status`、`slot_scan_count`
- 证据比对：`clone_check`、`clone_score`、`clone_match_seconds`、`clone_matched_evidence_id`

## 9. 退出码约定

- 运行失败（参数错误、IO 错误、检测阶段出现 invalid/error）返回非 0。
- `clone_check=suspect` 仅作为结果标注，不单独触发失败退出码。
