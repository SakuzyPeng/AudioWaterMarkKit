# Message Protocol and Key Slots

[中文](../../zh-CN/spec/message-and-key-slot.md)

## 1. Message Layout (16 bytes)

| Field | Bytes | Description |
| --- | --- | --- |
| `version` | 1 | Protocol version |
| `timestamp+slot` | 4 | `v1`: minutes only; `v2`: 27-bit minutes + 5-bit slot |
| `tag_packed` | 5 | 8 packed 5-bit characters |
| `hmac` | 6 | First 6 bytes of HMAC-SHA256 |

Current default version: `2` (`CURRENT_VERSION`).

## 2. v2 Packing Rules

```text
packed = (timestamp_minutes << 5) | key_slot
timestamp_minutes = packed >> 5
key_slot = packed & 0x1F
```

- `key_slot` range: `0..31`
- `timestamp_minutes` max: `2^27 - 1`
- `v1` compatibility: decode treats `key_slot` as `0`

## 3. Slot Diagnostics During Detect

CLI detect exports these slot diagnostics:

- `decode_slot_hint`: slot hinted by message header
- `decode_slot_used`: slot that actually succeeded for decode
- `slot_status`:
  - `matched`: hint slot matches decode slot
  - `recovered`: hint failed, another slot recovered decode
  - `mismatch`: no slot could decode
  - `missing_key`: hinted slot key is missing
  - `ambiguous`: multiple slots could decode
- `slot_scan_count`: number of configured slots scanned

## 4. clone-check Semantics

- `exact`: evidence matched by PCM SHA256
- `likely`: Chromaprint similarity passes threshold
- `suspect`: no strong match
- `unavailable`: evidence store or fingerprint path unavailable
