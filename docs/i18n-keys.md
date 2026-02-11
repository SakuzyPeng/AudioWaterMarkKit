# i18n Key Catalog (zh-CN / en-US)

本文件是 Win+mac UI 统一文案 key 的基线。新增用户可见文案时，先在这里登记 key，再分别落地到平台资源。

## 命名规范
- 页面: `page.<name>.*`
- 动作: `action.<verb>`
- 状态: `status.<domain>.*`
- 提示: `hint.*`
- 日志: `log.<kind>.*`

## 全局
- `nav.embed`
- `nav.detect`
- `nav.tags`
- `nav.key`
- `language.title`
- `language.zh`
- `language.en`
- `appearance.title`
- `appearance.system`
- `appearance.light`
- `appearance.dark`

## 运行时状态
- `status.key.name`
- `status.engine.name`
- `status.database.name`
- `status.refresh`
- `status.unavailable`
- `status.available`

## 日志语义 kind（文案与语义解耦）
- `log.process_started`
- `log.process_finished`
- `log.process_cancelled`
- `log.queue_cleared`
- `log.logs_cleared`
- `log.result_ok`
- `log.result_not_found`
- `log.result_invalid_hmac`
- `log.result_error`

## 参数占位规范
- 统一使用具名占位，如 `{count}`、`{slot}`、`{path}`。
- 同一个 key 的参数名在双端保持一致。

## 当前实现说明
- 语言持久化键: `app_settings.ui_language`（值域 `zh-CN | en-US`）。
- 本轮先完成 FFI 单一真源与双端切换入口；其余页面文案 key 将按本清单继续覆盖。
