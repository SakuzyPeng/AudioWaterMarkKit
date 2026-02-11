# 数据库 Schema

[English](../../en-US/spec/database-schema.md)

## 1. 数据库路径

- macOS / Linux：`~/.awmkit/awmkit.db`
- Windows：`%LOCALAPPDATA%\\awmkit\\awmkit.db`

## 2. 核心表

### `tag_mappings`

| 列 | 类型 | 说明 |
| --- | --- | --- |
| `username` | `TEXT PRIMARY KEY` | 用户名（`COLLATE NOCASE`） |
| `tag` | `TEXT NOT NULL` | 8 字符 Tag |
| `created_at` | `INTEGER NOT NULL` | Unix 秒 |

索引：`idx_tag_mappings_created_at(created_at DESC)`

### `audio_evidence`

| 列 | 类型 | 说明 |
| --- | --- | --- |
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | 证据 ID |
| `created_at` | `INTEGER NOT NULL` | 写入时间 |
| `file_path` | `TEXT NOT NULL` | 输出文件路径 |
| `tag` | `TEXT NOT NULL` | 解码 Tag |
| `identity` | `TEXT NOT NULL` | 解码 Identity |
| `version` | `INTEGER NOT NULL` | 消息版本 |
| `key_slot` | `INTEGER NOT NULL` | 槽位 |
| `timestamp_minutes` | `INTEGER NOT NULL` | UTC 分钟 |
| `message_hex` | `TEXT NOT NULL` | 16-byte 消息十六进制 |
| `sample_rate` | `INTEGER NOT NULL` | 采样率 |
| `channels` | `INTEGER NOT NULL` | 声道数 |
| `sample_count` | `INTEGER NOT NULL` | 样本数 |
| `pcm_sha256` | `TEXT NOT NULL` | PCM 指纹哈希 |
| `key_id` | `TEXT NOT NULL` | 密钥指纹短串 |
| `chromaprint_blob` | `BLOB NOT NULL` | Chromaprint 原始数据 |
| `fingerprint_len` | `INTEGER NOT NULL` | Chromaprint 长度 |
| `fp_config_id` | `INTEGER NOT NULL` | Chromaprint 配置 ID |

约束与索引：

- `UNIQUE(identity, key_slot, key_id, pcm_sha256)`
- `idx_audio_evidence_identity_slot_created(identity, key_slot, created_at DESC)`
- `idx_audio_evidence_slot_key_created(key_slot, key_id, created_at DESC)`

### `app_settings`

| 列 | 类型 | 说明 |
| --- | --- | --- |
| `key` | `TEXT PRIMARY KEY` | 配置键 |
| `value` | `TEXT NOT NULL` | 配置值 |
| `updated_at` | `INTEGER NOT NULL` | 更新时间 |

目前关键键：

- `active_key_slot`
- `ui_language`

### `key_slots_meta`

| 列 | 类型 | 说明 |
| --- | --- | --- |
| `slot` | `INTEGER PRIMARY KEY` | 槽位号 |
| `label` | `TEXT NOT NULL DEFAULT ''` | 槽位标签 |
| `created_at` | `INTEGER NOT NULL` | 创建时间 |
| `updated_at` | `INTEGER NOT NULL` | 更新时间 |

## 3. 访问原则

- CLI 与 Win/mac GUI 均通过 Rust 存储层与 FFI 访问数据库。
- UI 不再直接维护 SQL 语义，Rust 为单一真源。
