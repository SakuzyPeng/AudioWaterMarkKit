# AWMKit - 音频水印 CLI 工具

自包含的跨平台音频水印命令行工具，实现 128-bit 可验证水印消息格式。

## 特性

- **完全自包含**：内嵌 audiowmark 二进制，无需手动安装依赖
- **跨平台支持**：macOS ARM64, Windows x86_64
- **安全密钥存储**：macOS Keychain / Windows Credential Manager（失败时回退 DPAPI）
- **批量处理**：支持通配符和多文件操作
- **可验证水印**：128-bit 消息 + HMAC-SHA256 认证
- **多声道支持**：5.1 / 5.1.2 / 7.1 / 7.1.4 / 9.1.6 等格式

## 快速开始

### 安装

从 [GitHub Releases](https://github.com/SakuzyPeng/AudioWaterMarkKit/releases) 下载对应平台的发行版：

**macOS ARM64**:
```bash
tar -xzf awmkit-macos-arm64.tar.gz
cd awmkit-macos-arm64
./awmkit --version
```
如果 macOS 提示无法打开或来自未识别的开发者：
```bash
xattr -d com.apple.quarantine ./awmkit
```

**Windows x86_64**:
```powershell
Expand-Archive awmkit-windows-x86_64.zip
cd awmkit-windows-x86_64
.\awmkit.exe --version
```

### GUI（可选）

GUI 使用说明已单独整理：`docs/AWMKIT_GUI.md`

### 初始化

首次使用前，初始化密钥（自动保存到系统密钥库）：

```bash
awmkit init

# 输出:
# [OK] 生成 32 字节随机密钥
# [OK] 保存到系统密钥库 (Keychain)
# [OK] 初始化完成
```

### 嵌入水印

```bash
# 嵌入水印到单个文件
awmkit embed --tag SAKUZY input.wav

# 批量嵌入（通配符）
awmkit embed --tag SAKUZY *.wav

# 自定义输出路径和强度
awmkit embed --tag SAKUZY --output marked.wav --strength 15 input.wav
```

### 检测水印

```bash
# 检测并验证
awmkit detect output_wm.wav

# 输出:
# File: output_wm.wav
#   [OK] 检测成功
#   Identity: SAKUZY
#   Timestamp: 2026-02-03 14:30:22 UTC
#   HMAC: 验证通过

# JSON 格式输出
awmkit detect --json output_wm.wav
```

### 查看状态

```bash
awmkit status

# 输出:
# awmkit v0.1.2
#
# 密钥状态:
#   [OK] 已配置 (32 字节)
#   存储: macOS Keychain
#
# audiowmark 引擎:
#   [OK] 可用 (bundled)
#   版本: 0.6.5
#   路径: ~/.awmkit/bundled/bin/audiowmark
```

### 语言设置

- CLI：支持 `--lang zh-CN|en-US`，也会读取 `LANG/LC_ALL` 环境变量
- GUI：在 Status / Init 页的 Language 下拉中切换，自动保存
- 配置文件：`~/.awmkit/config.toml`（Windows: `%LOCALAPPDATA%\\awmkit\\config.toml`）

## 命令参考

### init - 初始化

生成随机密钥并保存到系统密钥库。

```bash
awmkit init
```

### key - 密钥管理

```bash
# 显示密钥信息（不泄露内容）
awmkit key show

# 从文件导入
awmkit key import keyfile.bin

# 导出到文件
awmkit key export backup.bin

# 轮换密钥
awmkit key rotate
```

### encode - 编码消息

将 Tag 编码为 16 字节水印消息（不涉及音频）。

```bash
awmkit encode --tag SAKUZY

# 输出:
# Encoded message (hex): 0101c1d05978131b57f7deb8e22a0b78
```

### decode - 解码消息

解码并验证 16 字节水印消息（不涉及音频）。

```bash
awmkit decode --hex 0101c1d05978131b57f7deb8e22a0b78

# 输出:
# Version: 1
# Timestamp: 2026-02-03 14:30:00 UTC
# Identity: SAKUZY
# Tag: SAKUZY_2
# HMAC: 验证通过
```

### embed - 嵌入水印

将水印嵌入到音频文件。

```bash
# 基本用法
awmkit embed --tag SAKUZY input.wav

# 批量处理
awmkit embed --tag SAKUZY file1.wav file2.wav file3.wav
awmkit embed --tag SAKUZY *.wav

# 自定义选项
awmkit embed --tag SAKUZY \
  --output custom_output.wav \
  --strength 15 \
  input.wav
```

**参数**:
- `--tag <TAG>`: 7 字符身份（自动生成校验位）
- `--output <PATH>`: 输出路径（默认 `<input>_wm.wav`）
- `--strength <N>`: 水印强度 1-30（默认 10）

### detect - 检测水印

从音频文件检测并验证水印。

```bash
# 基本用法
awmkit detect output_wm.wav

# 批量检测
awmkit detect *.wav

# JSON 输出（机器可读）
awmkit detect --json output_wm.wav

```

**参数**:
- `--json`: JSON 格式输出

### status - 系统状态

显示系统状态和配置信息。

```bash
# 基本状态
awmkit status

# 诊断模式
awmkit status --doctor

# 输出:
# 运行诊断检查...
#
# [OK] 密钥配置正常
# [OK] audiowmark 可执行
# [OK] 临时目录可写
#
# 所有检查通过!
```

### 全局参数

```bash
--verbose, -v        # 详细输出
--quiet, -q          # 静默模式
--audiowmark <PATH>  # 指定 audiowmark 回退路径（bundled 不可用时使用）
```

### tag - 用户名映射（可选）

用于把用户名映射为可用 Tag。默认不落盘，保存时会写入本地 SQLite 数据库。

```bash
# 只输出推荐 Tag（不保存）
awmkit tag suggest username

# 保存映射（用户名 -> Tag）
awmkit tag save username

# 指定 Tag 保存
awmkit tag save username --tag SAKUZY_X

# 查看已保存的映射
awmkit tag list

# 删除映射
awmkit tag remove username

# 清空所有映射
awmkit tag clear
```

**存储位置**：
- macOS/Linux: `~/.awmkit/awmkit.db`（表：`tag_mappings`）
- Windows: `%LOCALAPPDATA%\\awmkit\\awmkit.db`（表：`tag_mappings`）

## 消息格式

AWMKit 使用 128-bit 自描述水印消息：

```
┌──────────┬────────────┬──────────────────┬────────────┐
│ Version  │ Timestamp  │  UserTagPacked   │   HMAC     │
│  1 byte  │  4 bytes   │    5 bytes       │  6 bytes   │
└──────────┴────────────┴──────────────────┴────────────┘
              总计: 16 bytes = 128 bit
```

| 字段 | 说明 |
|------|------|
| Version | 协议版本（当前 0x01） |
| Timestamp | UTC Unix 分钟数（big-endian） |
| UserTagPacked | 8 字符 Base32（7 身份 + 1 校验） |
| HMAC | HMAC-SHA256 前 6 字节 |

### Tag 格式

Tag 使用 8 字符格式：**7 字符身份 + 1 字符校验位**

**字符集**（32 字符 Base32 变体，排除易混淆字符）：
```
A B C D E F G H J K M N P Q R S T U V W X Y Z 2 3 4 5 6 7 8 9 _
```

排除：`O`, `0`, `I`, `1`, `L`

**示例**：
- `SAKUZY` → `SAKUZY_2`（自动补齐校验位）
- `TESTID` → `TESTIDZ`
- `ABC` → `ABC____V`（自动补齐到 7 字符 + 校验位）

## 库使用

AWMKit 也可作为 Rust/Swift 库使用。

### Rust API

```toml
[dependencies]
awmkit = { git = "https://github.com/SakuzyPeng/AudioWaterMarkKit" }
```

```rust
use awmkit::{Audio, Tag, Message};

let key = b"your-32-byte-secret-key-here!!!!";

// 创建 Tag
let tag = Tag::new("SAKUZY")?;  // → "SAKUZY_2"

// 嵌入水印
let audio = Audio::new()?;
let msg = audio.embed_with_tag("input.wav", "output.wav", 1, &tag, key)?;

// 检测验证
if let Some(result) = audio.detect_and_decode("output.wav", key)? {
    println!("Identity: {}", result.identity());
    println!("Time: {}", result.timestamp_utc);
}
```

**多声道支持**:
```rust
use awmkit::ChannelLayout;

// 嵌入水印到 7.1.4 音频（自动检测布局）
audio.embed_multichannel("input_7.1.4.wav", "output.wav", &msg, None)?;

// 嵌入水印到 8ch 音频（手动指定为 5.1.2）
audio.embed_multichannel("input_8ch.flac", "output.wav", &msg, Some(ChannelLayout::Surround512))?;

// 检测多声道音频
let result = audio.detect_multichannel("output.wav", None)?;
for (idx, name, detect) in &result.pairs {
    if let Some(d) = detect {
        println!("{}: 检测成功, errors={}", name, d.bit_errors);
    }
}
```

### Swift API

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/SakuzyPeng/AudioWaterMarkKit", branch: "main")
]
```

```swift
import AWMKit

// 密钥管理（macOS Keychain）
let key = try AWMKeychain.require()  // 或 generateAndSaveKey()

// 创建 Tag
let tag = try AWMTag(identity: "SAKUZY")

// 嵌入水印
let audio = try AWMAudio()
try audio.embed(input: inputURL, output: outputURL, tag: tag, key: key)

// 检测验证
if let result = try audio.detectAndDecode(input: outputURL, key: key) {
    print("Identity: \(result.identity)")
    print("Time: \(result.date)")
}
```

### C FFI

AWMKit 提供 C 接口用于其他语言绑定：

```c
#include "awmkit.h"

// Tag 创建
AWMTag tag;
awm_tag_new("SAKUZY", &tag);

// 消息编码
uint8_t msg[16];
awm_message_encode(1, &tag, key, 32, msg);

// 音频嵌入
AWMAudio audio;
awm_audio_new(&audio);
awm_audio_embed(&audio, "input.wav", "output.wav", msg);
```

## 多声道支持

AWMKit 支持多声道音频的水印嵌入和检测，通过将音频拆分为立体声对分别处理。

### 支持的声道布局

| 布局 | 声道数 | 声道配置 | 立体声对 |
|------|--------|----------|----------|
| Stereo | 2 | FL FR | 1 |
| 5.1 | 6 | FL FR FC LFE BL BR | 3 |
| 5.1.2 | 8 | FL FR FC LFE BL BR TFL TFR | 4 |
| 7.1 | 8 | FL FR FC LFE BL BR SL SR | 4 |
| 7.1.4 | 12 | FL FR FC LFE BL BR SL SR TFL TFR TBL TBR | 6 |
| 9.1.6 | 16 | FL FR FC LFE BL BR SL SR FLC FRC TFL TFR TBL TBR TSL TSR | 8 |

**注意**：8 声道可能是 7.1 或 5.1.2 格式，默认按 7.1 处理。可通过 `ChannelLayout::Surround512` 手动指定。

### 支持的文件格式

| 格式 | 读取 | 写入 | 最大声道数 |
|------|------|------|------------|
| WAV | ✓ | ✓ | 无限制 |
| FLAC | ✓ | ✗ | 8 |

处理流程：多声道音频 → 拆分立体声对 → 每对嵌入水印 → 合并 → 输出 WAV

## 从源码构建

### 使用 Cargo

```bash
# 克隆仓库
git clone https://github.com/SakuzyPeng/AudioWaterMarkKit
cd awmkit

# 准备 bundled 资源（macOS arm64）
# 需要存在 bundled/audiowmark-macos-arm64.zip

# 构建 CLI
cargo build --bin awmkit --features full-cli --release

# 运行
./target/release/awmkit --version
```

### 使用 CI/CD

项目配置了 GitHub Actions 自动构建流程：

**触发方式 1：推送 tag**
```bash
git tag awmkit-0.1.0
git push origin awmkit-0.1.0
```

**触发方式 2：手动触发**
```bash
gh workflow run build-awmkit.yml \
  -f tag="awmkit-0.1.0" \
  -f prerelease=false
```

CI 会自动：
1. 下载对应平台的 audiowmark release
2. 组装 bundled zip 资源
3. 编译嵌入到 awmkit 二进制
4. 打包为 tar.gz（macOS）或 zip（Windows）
5. 创建 GitHub Release 并上传

详见 [AWMKIT_CI_PLAN.md](docs/AWMKIT_CI_PLAN.md)

### Features

```bash
# 仅库（不含 CLI）
cargo build --release

# 完整 CLI（bundled 优先，回退 --audiowmark/PATH）
cargo build --features full-cli --release

# C FFI（macOS 原生 App 推荐）
cargo build --features ffi,bundled --release

# 多声道支持
cargo build --features multichannel --release
```

### 本地最小命令（自包含优先）

```bash
# Rust CLI
cargo build --bin awmkit --features full-cli --release

# macOS 原生 App 的 Rust 库
cargo build --features ffi,bundled --release
```

> 说明：Tauri 栈已从仓库移除，不再提供 `src-tauri`/`ui` 构建链路。

### 消息协议

- 默认编码协议版本为 `v2`
- `v2` 的 4-byte 时间字段布局为：`27-bit UTC 分钟 + 5-bit key_slot`
- 当前单活密钥阶段，`key_slot` 固定为 `0`
- 解码保持兼容 `v1` 历史消息（`v1` 中该字段等价为纯 UTC 分钟）

## 技术细节

### 鲁棒性测试

水印基于 [audiowmark](https://github.com/swesterfeld/audiowmark) 扩频技术，经 30 项测试验证：

| 处理类型 | 测试结果 |
|----------|----------|
| AAC 64kbps 压缩 | ✓ 100% 检测率 |
| 多声道下混/上混 | ✓ 通过 |
| HRTF 双耳化处理 | ✓ 通过 |
| 音频叠加（1:4 比例） | ✓ 通过 |
| 重采样（22k/44k/96k） | ✓ 通过 |
| 位深转换（16/24/32bit） | ✓ 通过 |
| EQ/压缩/限制器 | ✓ 通过 |
| 回声/混响/噪声 | ✓ 通过 |
| 变速/变调 | ✗ 不支持 |

**通过率**：22/30（73%）
**推荐配置**：strength=10（默认），置信度 0.96，SNR 40.8dB

详细测试数据见 [EXPERIMENTS.md](./EXPERIMENTS.md)

### 安全考虑

- **48-bit HMAC**：对离线场景足够（在线攻击成本高）
- **校验位**：防止 OCR/手抄错误，非安全功能
- **Keychain**：使用 `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
- **密钥管理**：CLI 不存储明文密钥，仅通过系统密钥库访问
- **常量时间比较**：HMAC 验证使用常量时间比较防止时序攻击

### 自包含分发

awmkit 使用嵌入式二进制分发策略：

1. **编译时嵌入**：使用 `include_bytes!` 宏嵌入 `bundled/audiowmark-*.zip`
2. **运行时解压**：首次运行自动解压到 `~/.awmkit/bundled/bin/`
3. **校验和验证**：SHA256 校验确保二进制完整性
4. **平台特定**：每个平台仅包含对应的二进制（macOS ~206KB，Windows ~250KB）

最终发行版大小：macOS ~1.2MB，Windows ~1.5MB

## 目录结构

```
awmkit/
├── src/
│   ├── lib.rs              # 公共 API
│   ├── tag.rs              # Tag 编解码
│   ├── message.rs          # 消息编解码 + HMAC
│   ├── audio.rs            # audiowmark 封装
│   ├── bundled.rs          # Bundled 二进制管理
│   ├── multichannel.rs     # 多声道处理
│   ├── charset.rs          # Base32 字符集
│   ├── error.rs            # 错误类型
│   ├── ffi.rs              # C FFI
│   └── bin/awmkit/         # Rust CLI
│       ├── main.rs
│       ├── commands/       # 子命令实现
│       ├── keystore.rs     # 密钥存储
│       ├── output.rs       # 输出格式化
│       └── util.rs
├── bundled/                # 嵌入式二进制
│   ├── audiowmark-macos-arm64.zip
│   └── audiowmark-windows-x86_64.zip
├── include/
│   └── awmkit.h            # C 头文件
├── bindings/
│   └── swift/              # Swift Package
├── .github/workflows/
│   └── build-awmkit.yml    # CI/CD 配置
├── docs/
│   ├── AWMKIT_CLI_PLAN.md
│   └── AWMKIT_CI_PLAN.md
├── Cargo.toml
└── README.md
```

## 相关文档

- [AWMKIT_CLI_PLAN.md](docs/AWMKIT_CLI_PLAN.md) - CLI 设计文档
- [AWMKIT_CI_PLAN.md](docs/AWMKIT_CI_PLAN.md) - CI/CD 使用指南
- [BUILD.md](BUILD.md) - 从源码构建指南
- [EXPERIMENTS.md](EXPERIMENTS.md) - 鲁棒性测试详情
- [PRP.md](PRP.md) - 产品规范

## License

MIT License

## TODO

- [ ] Linux 平台支持（需要 Secret Service）
- [ ] macOS x86_64 支持
- [ ] API 层自动转换有损格式（mp3/aac/m4a）为 WAV 再检测
- [ ] 批量处理进度条优化
- [ ] 配置文件支持（~/.awmkit/config.toml）
