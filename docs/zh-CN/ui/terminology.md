# UI 术语表（双端统一）

本术语表用于统一 `macos-app` 与 `winui-app` 的用户可见文案与诊断文案。

## 核心术语

| English | 中文主词 | 说明 |
| --- | --- | --- |
| watermark | 水印 | 不用“标记/印记”替代 |
| detect | 检测 | 动词统一“检测” |
| embed | 嵌入 | 动词统一“嵌入” |
| key slot | 密钥槽位 | 不简写“槽” |
| evidence | 证据 | 列表场景可用“证据记录” |
| mapping | 映射 | 不与“绑定”混用 |
| fallback | 回退 | 默认层不直出内部 route |
| verify | 校验 | 对应 verify/verification |
| invalid | 无效 | 校验失败与非法状态统一词 |
| unavailable | 不可用 | 能力不可用统一词 |

## 文案模板

- 第一句：发生了什么（结果）。
- 第二句：下一步动作（可执行操作）。
- 第三句：必要时补充原因。

## 规则

- 默认层禁止暴露 `route=`、`status=`、`single_fallback`、`UNVERIFIED`。
- 技术细节进入“显示诊断”区域。
- 同一概念只允许一个主词。
