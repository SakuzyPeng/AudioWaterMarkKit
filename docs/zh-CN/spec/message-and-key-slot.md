# 消息协议与密钥槽位

[English](../../en-US/spec/message-and-key-slot.md)

## 1. 消息结构（16 bytes）

| 字段 | 字节数 | 说明 |
| --- | --- | --- |
| `version` | 1 | 协议版本 |
| `timestamp+slot` | 4 | `v1` 为分钟；`v2` 为 27-bit 分钟 + 5-bit 槽位 |
| `tag_packed` | 5 | 8 个 5-bit 字符打包 |
| `hmac` | 6 | HMAC-SHA256 前 6 字节 |

当前默认版本：`2`（见 `CURRENT_VERSION`）。

## 2. v2 打包规则

```text
packed = (timestamp_minutes << 5) | key_slot
timestamp_minutes = packed >> 5
key_slot = packed & 0x1F
```

- `key_slot` 范围：`0..31`
- `timestamp_minutes` 上限：`2^27 - 1`
- `v1` 兼容：解码时 `key_slot` 固定视为 `0`

## 3. 槽位与解码诊断

检测时 CLI 会输出槽位诊断字段：

- `decode_slot_hint`：消息头提示槽位
- `decode_slot_used`：实际成功解码使用的槽位
- `slot_status`：
  - `matched`：提示槽位即成功槽位
  - `recovered`：提示槽位失败，其他槽位恢复成功
  - `mismatch`：扫描后仍无法解码
  - `missing_key`：提示槽位没有可用密钥
  - `ambiguous`：多个槽位都能解码
- `slot_scan_count`：扫描了多少个已配置槽位

## 4. clone-check 结果语义

- `exact`：PCM SHA256 命中证据
- `likely`：Chromaprint 相似度达到阈值
- `suspect`：未命中或不够相似
- `unavailable`：证据库或指纹生成不可用
