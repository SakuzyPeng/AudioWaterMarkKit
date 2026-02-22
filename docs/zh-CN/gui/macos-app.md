# macOS 原生应用

[English](../../en-US/gui/macos-app.md)

## 1. 概览

macOS 端基于 SwiftUI，核心逻辑通过 Rust FFI 提供，覆盖：

- 嵌入页（输入源、嵌入设置、队列、日志）
- 检测页（检测信息、队列、日志、结果联动）
- 标签/数据库页（映射 + 证据管理）
- 密钥页（槽位、生成/删除、导入/导出、Hex 导入、标签、摘要）

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
- 预发布/本地构建通常未签名，若提示“已损坏”可执行：
  - `xattr -dr com.apple.quarantine /path/to/AWMKit.app`

## 4. 常见验证点

- 无密钥时，嵌入按钮禁用；检测允许执行但会产生未校验结果警告
- 切换槽位后，状态图标悬浮摘要立即更新
- 密钥导入支持 32 字节 `.bin` 与 64 位 Hex（可带 `0x` 前缀），槽位已有密钥时禁止覆盖
- 嵌入成功后自动写入证据，检测页 clone-check 可读取证据比对
