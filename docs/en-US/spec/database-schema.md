# Database Schema

[中文](../../zh-CN/spec/database-schema.md)

## 1. Database Path

- macOS / Linux: `~/.awmkit/awmkit.db`
- Windows: `%LOCALAPPDATA%\\awmkit\\awmkit.db`

## 2. Core Tables

### `tag_mappings`

| Column | Type | Description |
| --- | --- | --- |
| `username` | `TEXT PRIMARY KEY` | Username (`COLLATE NOCASE`) |
| `tag` | `TEXT NOT NULL` | 8-char tag |
| `created_at` | `INTEGER NOT NULL` | Unix seconds |

Index: `idx_tag_mappings_created_at(created_at DESC)`

### `audio_evidence`

| Column | Type | Description |
| --- | --- | --- |
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Evidence id |
| `created_at` | `INTEGER NOT NULL` | Insert timestamp |
| `file_path` | `TEXT NOT NULL` | Output file path |
| `tag` | `TEXT NOT NULL` | Decoded tag |
| `identity` | `TEXT NOT NULL` | Decoded identity |
| `version` | `INTEGER NOT NULL` | Message version |
| `key_slot` | `INTEGER NOT NULL` | Key slot |
| `timestamp_minutes` | `INTEGER NOT NULL` | UTC minutes |
| `message_hex` | `TEXT NOT NULL` | 16-byte message hex |
| `sample_rate` | `INTEGER NOT NULL` | Sample rate |
| `channels` | `INTEGER NOT NULL` | Channel count |
| `sample_count` | `INTEGER NOT NULL` | Sample count |
| `pcm_sha256` | `TEXT NOT NULL` | PCM hash |
| `key_id` | `TEXT NOT NULL` | Key fingerprint id |
| `chromaprint_blob` | `BLOB NOT NULL` | Raw chromaprint bytes |
| `fingerprint_len` | `INTEGER NOT NULL` | Chromaprint length |
| `fp_config_id` | `INTEGER NOT NULL` | Chromaprint config id |

Constraints and indexes:

- `UNIQUE(identity, key_slot, key_id, pcm_sha256)`
- `idx_audio_evidence_identity_slot_created(identity, key_slot, created_at DESC)`
- `idx_audio_evidence_slot_key_created(key_slot, key_id, created_at DESC)`

### `app_settings`

| Column | Type | Description |
| --- | --- | --- |
| `key` | `TEXT PRIMARY KEY` | Setting key |
| `value` | `TEXT NOT NULL` | Setting value |
| `updated_at` | `INTEGER NOT NULL` | Updated timestamp |

Current important keys:

- `active_key_slot`
- `ui_language`

### `key_slots_meta`

| Column | Type | Description |
| --- | --- | --- |
| `slot` | `INTEGER PRIMARY KEY` | Slot id |
| `label` | `TEXT NOT NULL DEFAULT ''` | Slot label |
| `created_at` | `INTEGER NOT NULL` | Created timestamp |
| `updated_at` | `INTEGER NOT NULL` | Updated timestamp |

## 3. Access Policy

- CLI and Win/mac GUIs access database via Rust stores and FFI.
- UI no longer owns SQL semantics; Rust is the single source of truth.
