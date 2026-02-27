# UI Glossary (Dual-UI Unified)

This glossary standardizes user-facing and diagnostic copy across `macos-app` and `winui-app`.

## Core Terms

| English | Canonical Chinese | Notes |
| --- | --- | --- |
| watermark | 水印 | Do not replace with 标记/印记 |
| detect | 检测 | Canonical verb for detection |
| embed | 嵌入 | Canonical verb for embedding |
| key slot | 密钥槽位 | Do not shorten to 槽 |
| evidence | 证据 | 证据记录 is allowed in list context |
| mapping | 映射 | Do not mix with 绑定 |
| fallback | 回退 | Do not expose internal route in default layer |
| verify | 校验 | Covers verify/verification |
| invalid | 无效 | For invalid or failed verification states |
| unavailable | 不可用 | For capability unavailability |

## Copy Template

- Sentence 1: what happened (result).
- Sentence 2: what to do next (action).
- Sentence 3: reason only when needed.

## Rules

- Default layer must not expose `route=`, `status=`, `single_fallback`, `UNVERIFIED`.
- Technical details go to the “Show diagnostics” area.
- One canonical term per concept.
