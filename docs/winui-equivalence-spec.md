# WinUI 等价规格冻结（基于 macOS 现实现）

## 1. 目标与范围

本文冻结 Windows WinUI 端与 macOS 原生端的页面逻辑等价规范，覆盖：

1. 嵌入页（Embed）
2. 检测页（Detect）
3. 数据库查询页（Tags/DB）
4. 顶栏运行状态图标（密钥/引擎/数据库）

对齐基线代码：

- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/Views/EmbedView.swift`
- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/ViewModels/EmbedViewModel.swift`
- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/Views/DetectView.swift`
- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/ViewModels/DetectViewModel.swift`
- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/Views/TagsView.swift`
- `/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/AWMKitApp.swift`

## 2. 全局交互规则

1. 页面状态由 ViewModel 驱动，禁止控件内部私有状态与 ViewModel 状态冲突。
2. 进度条显示百分比；任务完成后 3 秒快速收回到 0。
3. 待处理队列文件在每次处理后立即移除（成功/失败均移除）。
4. 日志容量上限 200，超出后从末尾裁剪。
5. 仅“已清空日志”日志项自动消失（3 秒）。
6. 同文案按钮靠图标区分语义；无障碍标签必须完整区分动作。

## 3. 嵌入页等价矩阵

### 3.1 输入与队列

1. `inputSource` 是独立输入源地址（文件或目录），不由队列反推。
2. 选择输入源（单选文件/目录）后：
   - 目录仅扫描当前层（不递归）
   - 仅导入 `wav/flac`（大小写不敏感）
3. 新输入结果追加到队尾，并按路径去重。
4. 去重发生时写日志：`已去重 N 个重复文件`。
5. 拖拽仅影响队列，不更新 `inputSource`。

### 3.2 按钮行为

1. 按钮顺序：`选择输入`、`选择输出`、`嵌入/停止`、`清空队列`、`清空日志`。
2. 清空队列仅清 `selectedFiles`，不清 `inputSource`。
3. 清空日志仅清 `logs`。
4. “清空”成功反馈：仅图标变绿，文案不变。

### 3.3 标签输入与映射

1. 输入栏为用户名输入，实时预览 Tag。
2. 若命中已存储映射，提示已存在，不覆盖。
3. 已存储映射下拉项显示 `username + tag`。
4. 嵌入成功后自动尝试保存当前映射（已存在则忽略）。

## 4. 检测页等价矩阵

### 4.1 检测流程与结果

1. `detect + decode` 两段式，状态值：`ok/not_found/invalid_hmac/error`。
2. `status=ok` 时附加 clone-check：
   - exact / likely / suspect / unavailable
3. 统计：标题右侧显示 `成功/总`，总数为 0 不显示。

### 4.2 检测信息卡（固定 6 行，无滚动）

行布局固定：

1. 行1（全宽）：文件
2. 行2（三列）：状态 | 匹配标记 | 检测模式
3. 行3（三列）：标签 | 身份 | 版本
4. 行4（三列）：检测时间 | 密钥槽位 | 位错误
5. 行5（三列）：检测分数 | 克隆校验 | 指纹分数
6. 行6（全宽）：错误信息

显示规则：

1. 所有值单行、`lineLimit(1)`、超长截断 + tooltip。
2. 无结果时显示字段名 + `-`。
3. 详情来源：选中日志优先；未选中默认最新；手动取消选中后可隐藏详情。

### 4.3 颜色语义

1. 状态：`ok=绿`、`not_found=灰`、`invalid_hmac=橙`、`error=红`
2. 匹配标记：`true=绿`、`false=灰`
3. 位错误：`0=绿`、`1..3=橙`、`>3=红`
4. 检测分数：`>=1.30绿`、`1.10..1.29橙`、`1.00..1.09黄`、`<1.00红`
5. 克隆校验：`exact绿`、`likely蓝`、`suspect橙`、`unavailable灰`
6. 指纹分数（越低越像）：`<=1绿`、`<=3蓝`、`<=7橙`、`>7红`
7. 错误信息：有值红，无值灰

### 4.4 日志联动

1. 仅结果日志可选中（`relatedRecordId != nil`）。
2. 流程日志不可选中。
3. 选中态：accent 描边 + 轻背景。
4. 日志卡支持搜索框（仅有日志时出现），匹配标题与详情。

## 5. 数据库查询页（原标签页）等价矩阵

### 5.1 页面结构

1. 双面板：左 `tag_mappings`，右 `audio_evidence`。
2. 顶部：全局搜索 + 作用域筛选（全部/映射/证据）+ 计数胶囊。
3. 默认每侧展示最近 200 条。

### 5.2 删除模式

1. 普通模式按钮：`添加标签`、`删除标签`、`删除证据`。
2. 删除模式互斥：同一时刻只能一种模式开启。
3. 删除模式内支持：全选/全不选/执行删除/退出删除。
4. 二次确认：必须输入选中数量才允许执行。
5. 删除后自动退出模式并刷新计数。

## 6. 顶栏状态图标等价规范

图标：密钥、引擎、数据库。

1. 颜色表达状态：ready/warning/error/unknown。
2. 悬浮提示：
   - 密钥：是否已配置、字节长度或失败原因
   - 引擎：是否可用、来源（bundled/PATH）
   - 数据库：映射总数、证据总数
3. 点击行为：触发一次手动状态刷新检测。

## 7. 禁止偏离项

1. 不得把输入源与队列重新耦合。
2. 不得把检测信息卡改回滚动列表。
3. 不得弱化删除二次确认机制。
4. 不得把 clone-check 状态改成影响成功检测退出码（CLI 约束保持）。

## 8. Windows 端实现建议（约束）

1. WinUI 3 + C#，MVVM。
2. UI 组件语义与状态命名保持与 macOS 一致。
3. FFI 调用统一封装在一层 Native Bridge，页面禁止直接 `DllImport`。
