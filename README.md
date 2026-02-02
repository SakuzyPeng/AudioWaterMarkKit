# AWMKit - Audio Watermark Kit

跨语言音频水印工具库，提供 128-bit 自描述、可验证的水印消息格式。

## 特性

- **消息层**：128-bit 消息编解码 + HMAC-48 认证
- **音频层**：封装 audiowmark，一键嵌入/检测
- **多声道**：支持 5.1 / 5.1.2 / 7.1 / 7.1.4 / 9.1.6 等格式
- **跨语言**：Rust / C FFI / Swift / (Python/WASM 待添加)
- **安全存储**：macOS Keychain 集成

## 消息格式

```
┌──────────┬────────────┬──────────────────┬────────────┐
│ Version  │ Timestamp  │  UserTagPacked   │   HMAC     │
│  1 byte  │  4 bytes   │    5 bytes       │  6 bytes   │
└──────────┴────────────┴──────────────────┴────────────┘
              总计: 16 bytes = 128 bit
```

| 字段 | 说明 |
|------|------|
| Version | 协议版本 (当前 0x01) |
| Timestamp | UTC Unix 分钟数 (big-endian) |
| UserTagPacked | 8 字符 Base32 (7 身份 + 1 校验) |
| HMAC | HMAC-SHA256 前 6 字节 |

## 安装

### Rust

```toml
[dependencies]
awmkit = { path = "/path/to/awmkit" }
```

### Swift

```swift
// Package.swift
dependencies: [
    .package(path: "/path/to/awmkit/bindings/swift")
]
```

### 前置依赖

音频操作需要 [audiowmark](https://github.com/swesterfeld/audiowmark)：

```bash
# macOS (x86_64 via Rosetta 或原生编译)
# 参考项目 vendor/ 目录的预编译版本

# Linux
sudo apt install audiowmark
```

## 快速开始

### Rust

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

### Swift

```swift
import AWMKit

// 密钥管理 (macOS Keychain)
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

### CLI (Swift)

推荐使用 Swift CLI，自带 Keychain 集成和 audiowmark：

```bash
# 下载分发包
tar -xzf awm-cli-1.0.0-macos.tar.gz
cd awm-cli

# 初始化密钥 (存入 macOS Keychain)
./awm init

# 查看状态
./awm status

# 嵌入水印
./awm embed input.wav output.wav --tag SAKUZY

# 检测验证
./awm detect output.wav

# 密钥管理
./awm key show
./awm key export backup.bin
./awm key import backup.bin
```

### CLI (Rust)

底层 CLI，用于消息编解码测试：

```bash
# 构建
cargo build --features cli --release

# 生成 Tag
./target/release/awm tag SAKUZY
# → SAKUZY_2

# 编码消息
./target/release/awm encode --tag SAKUZY_2 --key-file key.bin
# → 0101c1d05978131b57f7deb8e22a0b78

# 解码验证
./target/release/awm decode --hex 0101c1d05978131b57f7deb8e22a0b78 --key-file key.bin
# → Version: 1
# → Timestamp: 2026-01-18 12:41:00 (UTC)
# → Identity: SAKUZY
# → Status: Valid
```

### 鲁棒性测试

使用 `awm raw` 透传参数给 audiowmark：

```bash
# 测试不同强度
awm raw add input.wav out_s5.wav <hex> --strength 5
awm raw add input.wav out_s20.wav <hex> --strength 20

# 使用自定义密钥文件
awm raw gen-key test.key
awm raw add input.wav output.wav <hex> --key test.key

# 检测并输出 JSON
awm raw get output.wav --json result.json

# 比较验证
awm raw cmp output.wav <hex>
```

## API 参考

### Tag

```rust
// Rust
let tag = Tag::new("SAKUZY")?;      // 从身份创建 (自动补齐+校验)
let tag = Tag::parse("SAKUZY_2")?;  // 解析并验证
tag.identity()                       // → "SAKUZY"
tag.as_str()                         // → "SAKUZY_2"
tag.verify()                         // → true
```

```swift
// Swift
let tag = try AWMTag(identity: "SAKUZY")
let tag = try AWMTag(tag: "SAKUZY_2")
tag.identity  // → "SAKUZY"
tag.value     // → "SAKUZY_2"
tag.isValid   // → true
```

### Message

```rust
// Rust
let msg = Message::encode(1, &tag, key)?;
let msg = Message::encode_with_timestamp(1, &tag, key, ts_minutes)?;
let result = Message::decode(&msg, key)?;
let valid = Message::verify(&msg, key);
```

```swift
// Swift
let msg = try AWMMessage.encode(tag: tag, key: key)
let result = try AWMMessage.decode(msg, key: key)
let valid = AWMMessage.verify(msg, key: key)
```

### Audio

```rust
// Rust
let audio = Audio::new()?;                          // 自动搜索 audiowmark
let audio = Audio::with_binary("/path/to/bin")?;   // 指定路径
let audio = audio.strength(10).key_file("key");    // 配置

audio.embed(input, output, &msg)?;
audio.embed_with_tag(input, output, 1, &tag, key)?;
let result = audio.detect(input)?;
let decoded = audio.detect_and_decode(input, key)?;

// 多声道支持 (需要 multichannel feature)
use awmkit::ChannelLayout;
audio.embed_multichannel(input, output, &msg, None)?;           // 自动检测布局
audio.embed_multichannel(input, output, &msg, Some(ChannelLayout::Surround512))?;  // 指定 5.1.2
let result = audio.detect_multichannel(input, None)?;           // 检测所有声道对
```

```swift
// Swift
let audio = try AWMAudio()
let audio = try AWMAudio(binaryPath: "/path/to/audiowmark")
audio.setStrength(10)
audio.setKeyFile("/path/to/key")

try audio.embed(input: url, output: url, message: msg)
try audio.embed(input: url, output: url, tag: tag, key: key)
let result = try audio.detect(input: url)
let decoded = try audio.detectAndDecode(input: url, key: key)
```

### Keychain (Swift)

```swift
let keychain = AWMKeychain()  // 或 .shared

// 存储
try keychain.saveKey(data)
try keychain.importKey(from: url)
try keychain.generateAndSaveKey()

// 读取
let key = try keychain.loadKey()     // Data?
let key = try AWMKeychain.require()  // Data (不存在抛错)

// 其他
keychain.hasKey
try keychain.deleteKey()
try keychain.exportKey(to: url)
```

## 构建

### 快速开始

```bash
# Rust 库
cargo build --release

# 带 Rust CLI
cargo build --features cli --release

# 带 C FFI (生成 .dylib/.a)
cargo build --features ffi --release

# 带多声道支持 (WAV/FLAC)
cargo build --features ffi,multichannel --release

# 运行测试
cargo test --features ffi,multichannel

# Swift Package
cd bindings/swift
swift build
swift test

# Swift CLI (推荐)
cd cli-swift
./build.sh
.build/debug/awm --help

# 分发打包 (需要 audiowmark)
./dist.sh /path/to/audiowmark
# 生成: dist/awm-cli-1.0.0-macos.tar.gz
```

### 详细编译说明

参见 [BUILD.md](./BUILD.md)，包括：
- audiowmark 从源代码编译步骤
- macOS 和 Linux 编译指南
- 常见编译问题解决方案
- 特性和平台配置

## 分发包

分发包 `awm-cli-1.0.0-macos.tar.gz` 包含：

```
awm-cli/
├── awm              # 启动脚本
├── bin/
│   ├── awm          # 主程序
│   └── audiowmark   # 水印引擎 (x86_64)
├── lib/
│   ├── libawmkit.dylib
│   └── x86_64/      # audiowmark 依赖库
└── README.txt
```

解压即用，无需安装依赖。密钥存储在 macOS Keychain。

## 目录结构

```
awmkit/
├── src/
│   ├── lib.rs          # 公共 API
│   ├── tag.rs          # Tag 编解码
│   ├── message.rs      # 消息编解码
│   ├── audio.rs        # audiowmark 封装
│   ├── multichannel.rs # 多声道处理
│   ├── charset.rs      # Base32 字符集
│   ├── error.rs        # 错误类型
│   ├── ffi.rs          # C FFI
│   └── bin/awm.rs      # Rust CLI
├── include/
│   └── awmkit.h        # C 头文件
├── bindings/
│   └── swift/          # Swift Package
│       ├── Package.swift
│       └── Sources/AWMKit/
│           ├── Tag.swift
│           ├── Message.swift
│           ├── Audio.swift
│           ├── Keychain.swift
│           └── Error.swift
├── cli-swift/          # Swift CLI (推荐)
│   ├── Package.swift
│   ├── Sources/awm/main.swift
│   ├── build.sh        # 构建脚本
│   └── dist.sh         # 分发打包脚本
├── Cargo.toml
├── README.md
└── PRP.md              # 产品规范
```

## 字符集

Tag 使用 32 字符 Base32 变体（排除易混淆字符）：

```
A B C D E F G H J K M N P Q R S T U V W X Y Z 2 3 4 5 6 7 8 9 _
```

排除：`O`, `0`, `I`, `1`, `L`

## 鲁棒性

水印基于 [audiowmark](https://github.com/swesterfeld/audiowmark) 扩频技术，经 30 项测试验证：

| 处理类型 | 测试结果 |
|----------|----------|
| AAC 64kbps 压缩 | ✓ 100% 检测率 |
| 多声道下混/上混 | ✓ 通过 |
| HRTF 双耳化处理 | ✓ 通过 |
| 音频叠加 (1:4 比例) | ✓ 通过 |
| 重采样 (22k/44k/96k) | ✓ 通过 |
| 位深转换 (16/24/32bit) | ✓ 通过 |
| EQ/压缩/限制器 | ✓ 通过 |
| 回声/混响/噪声 | ✓ 通过 |
| 变速/变调 | ✗ 不支持 |

**通过率**: 22/30 (73%)
**推荐配置**: strength=10 (默认)，置信度 0.96，SNR 40.8dB

详细测试数据见 [EXPERIMENTS.md](./EXPERIMENTS.md)

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

> **注意**：8 声道可能是 7.1 或 5.1.2 格式，默认按 7.1 处理。可通过 `ChannelLayout::Surround512` 手动指定。

### 支持的文件格式

| 格式 | 读取 | 写入 | 最大声道数 |
|------|------|------|------------|
| WAV | ✓ | ✓ | 无限制 |
| FLAC | ✓ | ✗ | 8 |

### 处理流程

```
多声道音频 → 拆分立体声对 → 每对嵌入水印 → 合并 → 输出 WAV
```

### 使用示例

```rust
use awmkit::{Audio, ChannelLayout, Tag, Message};

let audio = Audio::new()?;
let key = b"your-32-byte-secret-key-here!!!!";
let tag = Tag::new("SAKUZY")?;
let msg = Message::encode(1, &tag, key)?;

// 嵌入水印到 7.1.4 音频 (自动检测布局)
audio.embed_multichannel("input_7.1.4.wav", "output.wav", &msg, None)?;

// 嵌入水印到 8ch 音频 (手动指定为 5.1.2)
audio.embed_multichannel("input_8ch.flac", "output.wav", &msg, Some(ChannelLayout::Surround512))?;

// 检测并查看各声道对结果
let result = audio.detect_multichannel("output.wav", None)?;
for (idx, name, detect) in &result.pairs {
    if let Some(d) = detect {
        println!("{}: 检测成功, errors={}", name, d.bit_errors);
    } else {
        println!("{}: 未检测到", name);
    }
}

// 获取最佳结果
if let Some(best) = result.best {
    let decoded = Message::decode(&best.raw_message, key)?;
    println!("Identity: {}", decoded.identity());
}
```

## 安全考虑

- **48-bit HMAC**：对离线场景足够（在线攻击成本高）
- **校验位**：防止 OCR/手抄错误，非安全功能
- **Keychain**：使用 `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
- **密钥管理**：库不存储密钥，由调用方负责

## License

MIT License

## TODO

- [ ] API 层自动转换有损格式（mp3/aac/m4a）为 WAV 再检测
- [ ] dylib 缓存验证使用哈希或版本号
- [ ] 检测时显示转换进度
- [ ] 批量处理进度条
