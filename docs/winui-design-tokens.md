# WinUI 视觉 Token 冻结（对齐 SwiftUI DesignSystem）

## 1. 来源

对齐源：`/Users/Sakuzy/code/rust/awmkit/macos-app/AWMKit/Sources/DesignSystem/DesignSystem.swift`

## 2. 布局与尺寸映射

### 2.1 窗口

1. 最小：1280 x 800
2. 默认：1280 x 880

### 2.2 间距

1. `grid = 24`
2. `card = 24`
3. `horizontal = 24`
4. `vertical = 24`
5. `section = 16`
6. `item = 12`
7. `compact = 8`

### 2.3 内边距

1. 卡片：12
2. 行：14
3. 按钮：`H16 V8`
4. 胶囊：`H10 V4`

### 2.4 圆角

1. 卡片：22
2. 按钮：18
3. 行：18
4. Toggle：16

## 3. 色彩语义映射

### 3.1 基础色

1. 成功：Green
2. 警告：Orange
3. 错误：Red
4. 链接/强调：Accent(Blue)

### 3.2 背景与边框（深浅模式）

1. Card 背景
   - Dark: `White 8%`
   - Light: `White 100%`
2. Row 背景
   - Dark: `White 8%`
   - Light: `Black 2%`
3. Button 背景
   - Dark: `White 12%`
   - Light: `White 95%`
4. Border
   - Dark: `White 25%`
   - Light: `Black 15%`
5. 标题条背景
   - Dark: `White 6%`
   - Light: `#F7F9FC`

### 3.3 检测卡字段语义色阈值

1. 状态：ok/ not_found/ invalid_hmac/ error
2. 位错误：0、1..3、>3
3. 检测分数：1.30、1.10、1.00 阈值
4. 克隆校验：exact/likely/suspect/unavailable
5. 指纹分数：1、3、7 阈值

## 4. 字体与排版

1. 按钮文案：`Subheadline + Semibold`
2. 胶囊文案：`Caption2 + Semibold`
3. 关键数值：等宽数字字体
4. 检测信息字段：单行显示，超长截断 + Tooltip

## 5. WinUI 控件映射建议

1. GlassCard -> `Border + Backdrop(Material: Mica/Acrylic)`
2. 胶囊按钮 -> `Button + CornerRadius=18 + Custom VisualState`
3. 进度条 -> 自定义画刷条 + 百分比右对齐
4. 双面板 -> `Grid` 两列 + 响应式折叠

## 6. 动画与反馈

1. 标准弹簧动画：WinUI 用 `Implicit Animations` 模拟
2. 按钮按压缩放：0.97
3. 日志/队列项插入删除：淡入 + 位移动画
4. 成功反馈：仅图标变绿，不改文字

## 7. 禁止项

1. 不允许使用默认控件样式导致“系统蓝灰扁平”外观回退。
2. 不允许检测详情区出现滚动条（固定 6 行信息卡）。
3. 不允许字段值换行导致行高膨胀。
