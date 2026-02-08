# WinUI 数据契约冻结（FFI / SQLite / 错误）

## 1. 目标

冻结 Windows WinUI 端对 Rust 内核的数据契约，保证：

1. 不改 Rust 协议与 ABI
2. WinUI 仅做桥接与呈现
3. macOS/CLI/Windows 三端语义一致

## 2. FFI C ABI（已存在，Windows 复用）

契约来源：

- `/Users/Sakuzy/code/rust/awmkit/include/awmkit.h`
- `/Users/Sakuzy/code/rust/awmkit/src/ffi.rs`

### 2.1 核心消息类型

1. `AWMResult`
   - `version: uint8`
   - `timestamp_utc: uint64`
   - `timestamp_minutes: uint32`
   - `key_slot: uint8`
   - `tag[9]`, `identity[8]`

2. `AWMDetectResult`
   - `found: bool`
   - `raw_message[16]`
   - `pattern[16]`
   - `has_detect_score: bool`
   - `detect_score: float`
   - `bit_errors: uint32`

3. `AWMCloneCheckResult`
   - `kind: enum(exact/likely/suspect/unavailable)`
   - `has_score + score(double)`
   - `has_match_seconds + match_seconds(float)`
   - `has_evidence_id + evidence_id(int64)`
   - `reason[128]`

### 2.2 核心函数

1. `awm_audio_new / awm_audio_new_with_binary / awm_audio_free`
2. `awm_audio_embed / awm_audio_detect`
3. `awm_message_encode / awm_message_decode`
4. `awm_clone_check_for_file(input, identity, key_slot, out)`
5. `awm_evidence_record_file(file_path, raw_message, key, key_len)`

### 2.3 错误码

1. 成功：`AWM_SUCCESS (0)`
2. 失败：负值（如 HMAC mismatch / no watermark / audiowmark not found）
3. WinUI 映射原则：
   - 可恢复错误 -> UI 日志 warning/error
   - 不可恢复错误 -> 任务终止并给出具体错误文本

## 3. C# P/Invoke 映射规范

## 3.1 结构体映射

1. `StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)`
2. 固定数组字段使用 `MarshalAs(UnmanagedType.ByValArray, SizeConst = N)`
3. 固定字符串字段采用 `byte[]/sbyte[]` 手动转 UTF-8/ANSI，避免截断歧义

## 3.2 句柄生命周期

1. `AWMAudioHandle*` 封装为 `SafeHandle`。
2. 每个 ViewModel 共享一个桥接实例，禁止频繁 new/free。
3. 应用退出时统一释放。

## 3.3 线程模型

1. FFI 调用在线程池执行。
2. 回调 UI 状态必须切回 UI 线程。
3. 批处理取消通过 `CancellationToken`，不强制中断 native 调用。

## 4. SQLite 契约（同一文件）

数据库路径：

1. Windows：`%LOCALAPPDATA%\\awmkit\\awmkit.db`（若无则回退 `%APPDATA%`）
2. macOS：`~/.awmkit/awmkit.db`

来源：

- `/Users/Sakuzy/code/rust/awmkit/src/app/tag_store.rs`
- `/Users/Sakuzy/code/rust/awmkit/src/app/evidence_store.rs`

### 4.1 `tag_mappings`

字段：

1. `username TEXT PRIMARY KEY COLLATE NOCASE`
2. `tag TEXT NOT NULL`
3. `created_at INTEGER NOT NULL`

索引：`idx_tag_mappings_created_at(created_at DESC)`

### 4.2 `audio_evidence`

字段：

1. `id INTEGER PK AUTOINCREMENT`
2. `created_at INTEGER`
3. `file_path/tag/identity/version/key_slot/timestamp_minutes/message_hex`
4. `sample_rate/channels/sample_count`
5. `pcm_sha256`
6. `chromaprint_blob`
7. `fingerprint_len`
8. `fp_config_id`

约束：`UNIQUE(identity, key_slot, pcm_sha256)`

索引：`idx_audio_evidence_identity_slot_created(identity, key_slot, created_at DESC)`

## 5. 查询与治理接口契约（Windows 实现需对齐）

### 5.1 标签映射

1. 列表：按 `username COLLATE NOCASE ASC`
2. 新增/更新：同用户名 upsert
3. 删除：按 username（大小写不敏感）

### 5.2 音频证据

1. 列表：`created_at DESC`，默认 limit 200
2. 筛选：`identity/tag/key_slot`
3. 删除：按 `id` 单条与批量过滤删除

## 6. clone-check 判定契约

1. 先 `sha256` 命中 -> `exact`
2. 否则指纹匹配：
   - `score <= 7 && duration >= 6s` -> `likely`
   - 否则 `suspect`
3. 无可用证据或不可计算 -> `unavailable`

## 7. 兼容性与边界

1. 消息协议默认 v2，保留 v1 解码兼容。
2. `key_slot` 当前默认 0（单活密钥阶段）。
3. WinUI 本阶段不得新增 ABI 字段。

## 8. 安全与隐私

1. 数据库存储本地，不上传。
2. 证据记录包含路径与 hash，UI 需提示本地敏感数据属性。
3. 卸载支持可选清理 `%USERPROFILE%\\.awmkit`。
