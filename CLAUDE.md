# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

AWMKit 是一个跨语言的音频水印消息编解码库，实现 128-bit 自描述、可验证的水印消息格式。核心用 Rust 实现，通过 C FFI 提供 Swift/ObjC 绑定，同时提供基于 Tauri 2 的桌面 GUI 应用。

## 常用命令

### Rust 构建与测试

```bash
# 构建库
cargo build --release

# 运行所有测试
cargo test

# 运行带 FFI 的测试
cargo test --features ffi

# 运行带 CLI 的测试
cargo test --features cli

# 运行带 app 层的测试
cargo test --features app

# 运行单个测试
cargo test test_name

# 检查代码
cargo clippy --all-features
```

### GUI 开发 (Tauri + React)

```bash
# 安装前端依赖
cd ui && npm install

# 启动 Tauri 开发模式（自动启动 Vite HMR + Tauri WebView）
cd src-tauri && cargo tauri dev

# 仅启动前端开发服务（端口 1420）
cd ui && npm run dev

# 构建前端（自动运行 i18n 键检查）
cd ui && npm run build

# 检查国际化键一致性
cd ui && npm run check:i18n

# 类型检查
cd ui && npx tsc --noEmit
```

### Swift 绑定

```bash
# 构建 Swift 包
cd bindings/swift && swift build

# 运行 Swift 测试
cd bindings/swift && swift test
```

### Swift CLI

```bash
# 构建 CLI
cd cli-swift && ./build.sh

# 创建分发包
cd cli-swift && ./dist.sh
```

## 架构

### 消息格式 (16 bytes)

```
[Version(1)] [Timestamp(4)] [TagPacked(5)] [HMAC(6)]
```

- **Version**: 当前 0x01
- **Timestamp**: big-endian UTC 分钟数
- **TagPacked**: 8×5bit = 40bit = 5 bytes
- **HMAC**: HMAC-SHA256 前 6 字节

### Tag 格式

8 字符 = 7 字符身份 + 1 字符校验位，使用 32 字符集（排除 O/0/I/1/L）。

### 整体架构

```
┌─ GUI (Tauri 2) ──────────────────────────┐
│  React 19 + TypeScript + Vite            │
│  HeroUI 组件库 (@heroui/react)           │
│  4 标签页: 嵌入 / 检测 / 状态 / 标签    │
└──────────── Tauri IPC ───────────────────┘
                  ↓
┌─ Rust App 层 (src/app/, feature: app) ───┐
│  i18n / keystore / tag_store /           │
│  audio_engine / settings / bundled       │
└──────────────────────────────────────────┘
                  ↓
┌─ Rust 核心库 ────────────────────────────┐
│  charset / tag / message / audio / ffi   │
└──────────────────────────────────────────┘
                  ↓
┌─ 跨语言绑定 ─────────────────────────────┐
│  C FFI → Swift/AWMKit → Swift CLI        │
│       → C 头文件 (include/awmkit.h)      │
└──────────────────────────────────────────┘
```

### Rust 核心模块

- `src/charset.rs` - 32 字符 Base32 变体字符集
- `src/tag.rs` - Tag 编解码 + 校验位计算
- `src/message.rs` - 消息编解码 + HMAC-SHA256
- `src/audio.rs` - audiowmark 命令行封装
- `src/ffi.rs` - C FFI 导出接口 (feature: ffi)
- `src/multichannel.rs` - 多声道水印处理

### Rust App 层 (feature: app)

为 GUI 提供后端功能的模块，位于 `src/app/`：

- `keystore.rs` - 系统密钥安全存储（macOS Keychain / Windows Credential Manager）
- `tag_store.rs` - 用户-标签映射管理（SQLite 持久化于 `~/.awmkit/awmkit.db` 的 `tag_mappings` 表）
- `i18n.rs` - Fluent 国际化框架集成（支持 en-US 和 zh-CN）
- `audio_engine.rs` - audiowmark 命令行封装（GUI 专用，处理批量操作和进度回调）
- `settings.rs` - 配置管理（TOML 持久化于 `~/.awmkit/config.toml`）
- `bundled.rs` - 内嵌 audiowmark 二进制管理（zstd 压缩，运行时解压）
- `maintenance.rs` - 维护功能（清除缓存、重置配置）
- `error.rs` - App 层统一错误类型

### Tauri 后端 (src-tauri/)

`src-tauri/src/main.rs` 通过 `#[tauri::command]` 暴露 14 个 IPC 命令供前端调用，包括：
`get_i18n_bundle`, `get_status`, `init_key`, `embed_files`, `detect_files`, `list_tags`, `save_tag`, `remove_tag` 等。

### 前端 (ui/)

React 19 + TypeScript + Vite 应用，使用 HeroUI 组件库 (@heroui/react)：

- `src/App.tsx` - 根组件，Tab 路由和全局状态管理
- `src/pages/` - 4 个页面：EmbedPage, DetectPage, StatusPage, TagPage
- `src/lib/api.ts` - Tauri IPC 调用封装层，所有后端通信入口
- `src/styles/tokens.css` - 设计令牌（颜色、字体、间距）
- `src/types/ui.ts` - TypeScript 类型定义
- `scripts/check-i18n-keys.mjs` - 检查国际化键是否在所有语言文件中定义

### 国际化 (i18n/)

使用 Fluent 格式（.ftl 文件），支持 en-US 和 zh-CN。UI 文本键以 `ui-` 前缀，CLI 文本以 `cli-` 前缀。

Swift 绑定位于 `bindings/swift/`，CLI 位于 `cli-swift/`。

## 代码规范

项目使用严格的 Clippy 配置（见 Cargo.toml lints 部分），禁止：
- `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable`
- 必须处理所有 Result
- 必须为所有公共和私有 API 编写文档注释（missing_docs_in_private_items）

HMAC 验证必须使用常量时间比较（防止时序攻击）。

### 二进制构建

项目包含多个二进制命令：
- `awmkit` - 完整 CLI（需要 `full-cli` feature）
- `awm` - 默认二进制（基础库模式）
- `FTSC-detect` / `FTSC-embed` - 简化版命令（需要 `simple-cli` feature）

构建特定二进制：
```bash
# 完整 CLI（推荐用于分发）
cargo build --bin awmkit --features full-cli --release

# 简化版命令
cargo build --bin FTSC-detect --features simple-cli --release
cargo build --bin FTSC-embed --features simple-cli --release

# GUI 应用（包含 Tauri 后端）
cd src-tauri && cargo tauri build
```

## 外部依赖

音频嵌入/检测需要 `audiowmark` 二进制文件，搜索路径：
- `audiowmark`
- `/usr/local/bin/audiowmark`
- `/opt/homebrew/bin/audiowmark`
