# CLI Terminology (Task-Oriented)

This glossary standardizes default CLI output and `--verbose` diagnostics to avoid term drift.

## Core Rules

- User-facing copy prioritizes "result term + action term".
- One canonical term per concept. Add a compatibility alias only when necessary.
- New messages must reuse terms from this glossary.

## Core Terms (EN -> ZH)

| English | Canonical Chinese | Notes |
| --- | --- | --- |
| watermark | 水印 | Do not replace with "标记" or "印记" |
| detect | 检测 | Use as the canonical verb |
| embed | 嵌入 | Use as the canonical verb |
| key slot | 密钥槽位 | Do not shorten to "槽" |
| evidence | 证据记录 | "证据" is allowed in list context |
| mapping | 映射 | Do not replace with "绑定" |
| fallback | 回退 | "回退路径" is allowed in diagnostics |
| verify | 校验 | Covers verify/verification |
| invalid | 无效 | For validation or semantic invalid states |
| unavailable | 不可用 | For capability unavailability, not runtime failure |

## Copy Template

- Default user output:
  - Sentence 1: What happened (result).
  - Sentence 2: What to do next (actionable command or check).
  - Sentence 3: Add reason only when required.
- `--verbose` diagnostics:
  - Keep full technical detail.
  - Terms must stay consistent with this glossary.

## Prohibited Patterns

- Exposing internal field names in default copy (for example `slot_hint`, `scan_count`).
- Error-only copy with no next-step action.
- Mixed canonical terms for one concept in the same language (for example "映射/绑定").
