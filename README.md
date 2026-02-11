# AWMKit

[English](./README.en.md)

AWMKit 是一个围绕音频水印场景构建的跨平台工具集，包含 Rust 核心库、CLI、macOS 原生应用、WinUI 应用与 Swift 绑定。

## 能力概览

- 音频水印消息协议（128-bit，HMAC 校验，`v2` 槽位语义）
- CLI 全流程：密钥、标签映射、嵌入、检测、证据管理
- macOS / Windows 原生 GUI（共享 Rust FFI 核心）
- SQLite 持久化：映射、证据、应用设置

## 平台矩阵

| 组件 | macOS (arm64) | Windows (x64) |
| --- | --- | --- |
| `awmkit` CLI | 支持 | 支持 |
| macOS App | 支持 | 不适用 |
| WinUI App | 不适用 | 支持 |
| Rust FFI (`awmkit`/`awmkit_native`) | 支持 | 支持 |

## 快速开始

- 文档总览（中文）：[`docs/zh-CN/INDEX.md`](./docs/zh-CN/INDEX.md)
- CLI 使用指南：[`docs/zh-CN/cli/usage.md`](./docs/zh-CN/cli/usage.md)
- 构建矩阵：[`docs/zh-CN/build/build-matrix.md`](./docs/zh-CN/build/build-matrix.md)

## 文档导航

- CLI：[`docs/zh-CN/cli/usage.md`](./docs/zh-CN/cli/usage.md)
- GUI（macOS）：[`docs/zh-CN/gui/macos-app.md`](./docs/zh-CN/gui/macos-app.md)
- GUI（WinUI）：[`docs/zh-CN/gui/winui-app.md`](./docs/zh-CN/gui/winui-app.md)
- 构建与发布：[`docs/zh-CN/build/build-matrix.md`](./docs/zh-CN/build/build-matrix.md)、[`docs/zh-CN/build/ci-artifacts.md`](./docs/zh-CN/build/ci-artifacts.md)
- 协议与数据：[`docs/zh-CN/spec/message-and-key-slot.md`](./docs/zh-CN/spec/message-and-key-slot.md)、[`docs/zh-CN/spec/database-schema.md`](./docs/zh-CN/spec/database-schema.md)
- 故障排查：[`docs/zh-CN/troubleshooting/common-issues.md`](./docs/zh-CN/troubleshooting/common-issues.md)

## 上游引擎（audiowmark）

- AWMKit 的音频嵌入/检测能力依赖上游项目 [`audiowmark`](https://github.com/swesterfeld/audiowmark)。
- 运行时采用 bundled executable 方式（打包并解压可执行文件），而非库静/动态链接。
- 构建与发布中使用的 `audiowmark` 二进制来源与 CI 流程见：[`docs/zh-CN/build/ci-artifacts.md`](./docs/zh-CN/build/ci-artifacts.md)。
- 许可证关系：本仓库采用 GPLv3+，并遵循上游 `audiowmark` 的开源许可证要求。
- 致谢：感谢 `audiowmark` 作者与社区提供的音频水印基础能力。

## 许可证

本项目采用 [GNU GPLv3 or later](./LICENSE)。
