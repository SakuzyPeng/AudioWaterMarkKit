# AWMKit macOS 原生应用

基于 SwiftUI 的 macOS 原生音频水印工具，采用玻璃效果设计风格。

## 技术栈

- **SwiftUI** - 现代 UI 框架
- **XcodeGen** - 项目配置管理
- **AWMKit Swift 绑定** - Rust 核心库的 Swift 封装
- **玻璃效果设计** - 参考 AWMKitGlass 设计系统

## 项目结构

```
macos-app/
├── project.yml                    # XcodeGen 配置
├── AWMKit/
│   ├── Info.plist                 # 应用配置
│   ├── Sources/
│   │   ├── AWMKitApp.swift        # 应用入口
│   │   ├── ContentView.swift      # 主视图（侧边栏导航）
│   │   ├── DesignSystem/          # 设计系统
│   │   │   ├── DesignSystem.swift # 颜色、间距、阴影等
│   │   │   └── GlassModifier.swift# 玻璃效果修饰器
│   │   ├── Components/            # 可复用组件
│   │   │   ├── GlassCard.swift    # 玻璃卡片容器
│   │   │   └── FileDropZone.swift # 文件拖放区
│   │   └── Views/                 # 页面视图
│   │       ├── EmbedView.swift    # 嵌入水印页面
│   │       ├── DetectView.swift   # 检测水印页面
│   │       ├── StatusView.swift   # 系统状态页面
│   │       └── TagsView.swift     # 标签管理页面
│   └── Resources/                 # 资源文件
└── README.md                      # 本文件
```

## 开发步骤

### 1. 生成 Xcode 项目

```bash
cd macos-app

# 安装 XcodeGen（如果未安装）
brew install xcodegen

# 生成 .xcodeproj
xcodegen generate
```

### 2. 构建 Rust 库

```bash
cd ..
cargo build --release --features ffi,bundled
```

说明：`ffi,bundled` 会让 `AWMAudio()` 优先使用 bundled audiowmark；请确保仓库中存在 `bundled/audiowmark-macos-arm64.zip`。

### 3. 打开项目

```bash
open AWMKit.xcodeproj
```

在 Xcode 中按 `Cmd+R` 运行应用。

## 设计特点

### 玻璃效果（Glass Effect）

所有卡片和组件使用玻璃效果：
- 半透明背景（暗色模式: 8% 白色，亮色模式: 95% 白色）
- 模糊背景（`.glassEffect(.regular)`）
- 细边框和阴影
- 流畅动画（弹簧动画）

### 组件系统

- **GlassCard** - 玻璃效果卡片容器
- **FileDropZone** - 支持拖放的文件选择区
- **StatusRow** - 状态信息行（带图标和颜色状态）
- **DetectResultRow / TagEntryRow** - 结果列表行

### 布局

- 左侧边栏导航（4 个标签页）
- 主内容区使用 GlassCard 分区
- 统一的间距和内边距系统（DesignSystem）

## 功能模块

### 嵌入水印（EmbedView）
- 文件拖放/选择
- Tag 输入（7 字符身份）
- 水印强度调节（1-30）
- 批量处理进度展示

### 检测水印（DetectView）
- 批量文件检测
- 显示 Tag、时间戳
- HMAC 验证状态

### 系统状态（StatusView）
- 密钥状态检查
- AudioWmark 引擎状态
- 初始化密钥操作

### 标签管理（TagsView）
- 用户名 → Tag 映射
- 添加/删除标签
- 持久化存储

## TODO

- [ ] 集成 Rust 库（解决模块引用问题）
- [ ] 实现文件拖放的实际逻辑
- [ ] 添加进度条和加载动画
- [ ] 实现标签持久化存储（TagStore）
- [ ] 添加应用图标和资源
- [ ] 错误处理和用户提示
- [ ] 多语言支持（i18n）
- [ ] 偏好设置页面
