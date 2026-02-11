# macOS 原生应用

[English](../../en-US/gui/macos-app.md)

## 1. 概览

macOS 端基于 SwiftUI，核心逻辑通过 Rust FFI 提供，覆盖：

- 嵌入页（输入源、嵌入设置、队列、日志）
- 检测页（检测信息、队列、日志、结果联动）
- 标签/数据库页（映射 + 证据管理）
- 密钥页（槽位、生成/删除、标签、摘要）

## 2. 开发构建

```bash
# 1) 构建 Rust 动态库
cargo build --lib --features ffi,app,bundled --release

# 2) 生成 Xcode 工程
cd macos-app
xcodegen generate

# 3) 构建 App
xcodebuild \
  -project AWMKit.xcodeproj \
  -scheme AWMKit \
  -configuration Debug \
  -sdk macosx \
  build
```

## 3. 运行依赖

- bundled 模式依赖仓库内 `bundled/audiowmark-macos-arm64.zip`
- 数据库路径：`~/.awmkit/awmkit.db`
- 密钥存储与槽位管理由 Rust 层统一处理（UI 通过 FFI 调用）

## 4. 常见验证点

- 无密钥时，嵌入/检测主按钮禁用并提示跳转密钥页
- 切换槽位后，状态图标悬浮摘要立即更新
- 嵌入成功后自动写入证据，检测页 clone-check 可读取证据比对
