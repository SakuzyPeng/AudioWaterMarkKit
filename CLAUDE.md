# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

AWMKit 是一个跨语言的音频水印消息编解码库，实现 128-bit 自描述、可验证的水印消息格式。核心用 Rust 实现，通过 C FFI 提供 Swift/ObjC 与 WinUI 绑定。Tauri 栈已从仓库移除。

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

# 运行带 app 层的测试（含 multichannel + ffmpeg-decode）
cargo test --features app

# 运行单个测试
cargo test test_name

# 格式化与检查（提交前必须通过）
cargo fmt
cargo clippy --all-features
```

### Feature Flags 说明

- `ffi` - C FFI 导出（构建动态库供 Swift/WinUI 使用）
- `cli` - 基础 CLI（clap + hex）
- `app` - GUI 后端完整功能（keystore/tag_store/i18n/audio_engine/settings）
- `bundled` - 内嵌 audiowmark 二进制（zip 解压，运行时使用）
- `full-cli` - 完整 CLI = app + bundled + ffi + ffmpeg-decode + clap + glob + indicatif
- `multichannel` - 多声道处理（默认启用）
- `ffmpeg-decode` - FFmpeg 解码（app/full-cli 自动启用）

### macOS 应用开发

```bash
# 1. 构建 Rust FFI 库（macOS GUI 所需 feature 组合）
cargo build --release --features ffi,bundled,app

# 2. 生成 Xcode 项目（修改 project.yml 后需重新运行）
cd macos-app && xcodegen generate

# 3. 打开 Xcode
open AWMKit.xcodeproj
```

在 Xcode 中按 `Cmd+R` 运行。`bundled` feature 需要仓库中存在 `bundled/audiowmark-macos-arm64.zip`。

### Swift 绑定

```bash
# 构建 Swift 包
cd bindings/swift && swift build

# 运行 Swift 测试
cd bindings/swift && swift test
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
┌─ GUI (原生) ─────────────────────────────┐
│  macOS SwiftUI / Windows WinUI           │
│  4 标签页: 嵌入 / 检测 / 标签 / 密钥     │
└─────────── FFI Bridge ───────────────────┘
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
│  C FFI → Swift/AWMKit                     │
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

### 国际化 (i18n/)

使用 Fluent 格式（.ftl 文件），支持 en-US 和 zh-CN。UI 文本键以 `ui-` 前缀，CLI 文本以 `cli-` 前缀。

Swift 绑定位于 `bindings/swift/`。

## 代码规范

项目使用严格的 Clippy 配置（见 Cargo.toml lints 部分），禁止：
- `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable`
- 必须处理所有 Result
- 必须为所有公共和私有 API 编写文档注释（missing_docs_in_private_items）

HMAC 验证必须使用常量时间比较（防止时序攻击）。

FFI 变更需同步更新 `include/awmkit.h`。

## 提交规范

遵循 Conventional Commits：`feat:`, `fix:`, `docs:`, `perf:`, `test:`, `api:`，可加 scope（如 `feat(cli): add key export`）。
PR 描述和 issue 讨论默认使用中文。

### 二进制构建

项目包含二进制命令：
- `awmkit` - 完整 CLI（需要 `full-cli` feature）

构建特定二进制：
```bash
# 完整 CLI（推荐用于分发）
cargo build --bin awmkit --features full-cli --release

# GUI 应用（原生）
# macOS: xcodebuild -project macos-app/AWMKit.xcodeproj ...
# Windows: dotnet build winui-app/AWMKit/AWMKit.csproj ...
```

## 外部依赖

音频嵌入/检测需要 `audiowmark` 二进制文件。audiowmark 无官方发行包，需自行编译。
- `bundled` feature：运行时自动解压内嵌二进制（`~/.awmkit/bundled/bin/audiowmark`），解压失败直接报错，不再 fallback 到系统路径
- 非 `bundled`：搜索 `audiowmark`（PATH）；开发者可通过 `--audiowmark <PATH>` 指定路径
