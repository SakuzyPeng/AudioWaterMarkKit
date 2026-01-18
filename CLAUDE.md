# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

AWMKit 是一个跨语言的音频水印消息编解码库，实现 128-bit 自描述、可验证的水印消息格式。核心用 Rust 实现，通过 C FFI 提供 Swift/ObjC 绑定。

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

# 运行单个测试
cargo test test_name

# 检查代码
cargo clippy --all-features
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

### Rust 核心模块

- `src/charset.rs` - 32 字符 Base32 变体字符集
- `src/tag.rs` - Tag 编解码 + 校验位计算
- `src/message.rs` - 消息编解码 + HMAC-SHA256
- `src/audio.rs` - audiowmark 命令行封装
- `src/ffi.rs` - C FFI 导出接口 (feature: ffi)

### 跨语言架构

```
Rust 核心 → C FFI → Swift/AWMKit → Swift CLI
                  → C 头文件 (include/awmkit.h)
```

Swift 绑定位于 `bindings/swift/`，CLI 位于 `cli-swift/`。

## 代码规范

项目使用严格的 Clippy 配置（见 Cargo.toml），禁止：
- `unwrap`, `expect`, `panic`, `todo`
- 必须处理所有 Result

HMAC 验证必须使用常量时间比较（防止时序攻击）。

## 外部依赖

音频嵌入/检测需要 `audiowmark` 二进制文件，搜索路径：
- `audiowmark`
- `/usr/local/bin/audiowmark`
- `/opt/homebrew/bin/audiowmark`
