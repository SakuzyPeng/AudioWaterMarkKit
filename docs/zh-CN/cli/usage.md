# CLI 使用指南

[English](../../en-US/cli/usage.md)

## 1. 安装与构建

### 从 Release 使用

- macOS：下载 `awmkit-macos-arm64.tar.gz`，解压后运行 `./awmkit`
- Windows：下载 `awmkit-windows-x86_64.zip`，解压后运行 `awmkit.exe`

当前包内仅包含单个 launcher 可执行文件，首次运行会自动解压运行时到：
- macOS：`~/.awmkit/runtime/<payload-hash>/`
- Windows：`%LOCALAPPDATA%\\awmkit\\runtime\\<payload-hash>\\`

### 从源码构建（推荐命令）

```bash
cargo build --bin awmkit-core --features full-cli --release
cargo build --bin awmkit --features launcher --release
```

`full-cli` 用于构建真实 CLI core，`launcher` 用于构建单文件入口。

## 2. 支持格式与布局

- 输入音频：`wav` / `flac` / `mp3` / `ogg` / `opus` / `m4a` / `alac` / `mp4` / `mkv` / `mka` / `ts` / `m2ts` / `m2t`
- 输出音频：`wav`（当前仅支持 WAV 输出；若 `--output` 不是 `.wav` 会直接报错）
- ADM/BWF（一期）：`embed` 会自动识别 `RIFF/RF64/BW64` 中的 ADM/BWF 元数据并走保真路径；若保真链路失败会直接报错（不降级）；`detect` 暂不支持 ADM 专项检测
- 声道布局：`auto`、`stereo`、`surround51`、`surround512`、`surround71`、`surround714`、`surround916`
- 多声道默认路由（smart）：`FL/FR` 与环绕声道按成对嵌入，`FC` 按单声道嵌入（dual-mono），`LFE` 默认跳过；未知/自定义布局回退为顺序配对，若奇数声道则最后一路按单声道处理并给出警告
- 多声道路由执行：内部使用 Rayon 并行处理 RouteStep，并按 step 索引确定性归并结果（不新增 CLI 参数）

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
- `cache clean`：清理 launcher 运行时缓存（`--db` 可选删除数据库/配置）

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
- 命中已含水印的输入文件会自动跳过，并在批处理结束后汇总告警。
- `evidence list/show` 与 `evidence --json` 聚焦当前可用证据字段（映射、指纹与统计信息）。

## 8. 检测 JSON 关键字段

`awmkit detect --json` 输出中常见字段：

- 结果字段：`status`、`tag`、`identity`、`version`、`key_slot`
- 解码槽位诊断：`decode_slot_hint`、`decode_slot_used`、`slot_status`、`slot_scan_count`
- 证据比对：`clone_check`、`clone_score`、`clone_match_seconds`、`clone_matched_evidence_id`

## 9. 退出码约定

- 运行失败（参数错误、IO 错误、检测阶段出现 invalid/error）返回非 0。
- `clone_check=suspect` 仅作为结果标注，不单独触发失败退出码。

## 10. 运行时清理

仅删除 `awmkit` / `awmkit.exe` 不会删除已解压运行时。

```bash
# 只清理 runtime 解压缓存
awmkit cache clean --yes

# 清理 runtime + 数据库/配置
awmkit cache clean --db --yes
```

说明：
- `cache clean` 不会自动删除密钥。
- 若检测到仍有已配置槽位，会打印非阻塞提醒。
