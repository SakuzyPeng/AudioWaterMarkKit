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

```bash
# Rust 库
cargo build --release

# 带 Rust CLI
cargo build --features cli --release

# 带 C FFI (生成 .dylib/.a)
cargo build --features ffi --release

# 带多声道支持 (WAV/FLAC)
cargo build --features multichannel --release

# 运行测试
cargo test --features multichannel

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

## 鲁棒性测试结果

基于 [audiowmark](https://github.com/swesterfeld/audiowmark) 的水印引擎，在 **120s 切片 + AAC 64kbps** 压缩场景下的测试结果：

### Strength vs 检测置信度 vs SNR

| strength | 检测率 | 置信度 | SNR (dB) | 评价 |
|----------|--------|--------|----------|------|
| 4 | 100% | 0.40 | 48.8 | ✗ 置信度不足 |
| 5 | 100% | 0.48 | 46.8 | ✗ 置信度不足 |
| 6 | 100% | 0.57 | 45.3 | ⚠ 勉强可用 |
| 7 | 100% | 0.67 | 43.9 | ⚠ 勉强可用 |
| **8** | **100%** | **0.76** | **42.8** | ◎ 偏重音质 |
| **9** | **100%** | **0.86** | **41.8** | ◎ 平衡 |
| **10** | **100%** | **0.96** | **40.8** | ★ **推荐** |
| 11 | 100% | 1.05 | 40.0 | ● 高鲁棒 |
| 12 | 100% | 1.14 | 39.3 | ● 高鲁棒 |
| 15 | 100% | 1.37 | 37.9 | ● 高鲁棒 |
| 20 | 100% | 1.85 | 34.9 | ● 极高鲁棒 |

### 指标说明

- **置信度 (Confidence)**：audiowmark 检测的 sync 分数，>0.5 可靠，>0.7 良好，>1.0 极强
- **SNR (信噪比)**：原始音频与水印音频的信噪比，>40dB 优秀，>35dB 良好，>30dB 可接受

### 推荐配置

| 场景 | strength | 置信度 | SNR | 说明 |
|------|----------|--------|-----|------|
| 追求音质 | 8 | 0.76 | 42.8 dB | 最小可听影响 |
| **平衡 (默认)** | **10** | **0.96** | **40.8 dB** | **推荐** |
| 追求鲁棒 | 12 | 1.14 | 39.3 dB | 适合多次转码 |
| 极端环境 | 15+ | >1.3 | <38 dB | 低码率/多次压缩 |

### 切片时长影响 (strength=10, AAC 64k)

| 切片时长 | 检测率 | 置信度 |
|----------|--------|--------|
| 30s | 100% | 0.92 |
| 45s | 100% | 0.92 |
| 60s | 100% | 0.93 |
| 90s | 100% | 0.91 |
| 120s | 100% | 0.94 |

> **结论**：默认 strength=10 在 AAC 64kbps 场景下表现优秀，30s 以上切片均可 100% 检测。

### 误报率测试

对无水印音频进行检测，验证误报率：

| 音频类型 | 最高置信度 | 目标消息匹配 |
|----------|------------|--------------|
| 无水印 (AAC 64k) | 0.26 | 无 |
| 有水印 (s=10, AAC 64k) | 0.91 | 有 |

- **区分比**: 3.5x (0.91 / 0.26)
- **推荐阈值**: 0.5 (误报率 ≈ 0%)

### 消息格式测试

测试不同 128-bit 消息的兼容性：

| 消息 (hex) | 无压缩 | AAC 64k |
|------------|--------|---------|
| `deadbeefcafebabe...` | 1.39 | 0.96 |
| `aaaaaaaaaaaaaaaa...` | 1.39 | 0.92 |
| `0123456789abcdef...` | 1.39 | 0.91 |
| `00000000000000000...` | 1.39 | 0.94 |
| `ffffffffffffffff...` | 1.39 | 0.92 |

> **结论**：任意 128-bit 消息均可正常工作，包括全 0 和全 F。

### 多水印叠加测试

对同一音频依次嵌入 5 个不同水印：

| 嵌入顺序 | 无压缩检测 | AAC 64k 检测 |
|----------|------------|--------------|
| M1 (第一个) | ✗ | ✗ |
| M2 | ✓ 6.82 | ✓ 4.56 |
| M3 | ✓ 7.04 | ✓ 4.37 |
| M4 | ✗ | ✗ |
| M5 (最后) | ✓ 6.67 | ✓ 4.42 |

> **结论**：多水印会相互干扰，仅部分可检测 (3/5)。audiowmark 设计为单水印使用，不支持叠加。

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

### 声道对检测结果

以 7.1.4 (12ch) 为例，各声道对的检测置信度：

| 声道对 | 置信度 | 说明 |
|--------|--------|------|
| FL+FR | 1.39 | ✓ 主声道，最佳 |
| FC+LFE | 0.45 | ⚠ 中置+低音炮，置信度较低 |
| BL+BR | 1.35 | ✓ 后置环绕 |
| SL+SR | 1.38 | ✓ 侧环绕 |
| TFL+TFR | 1.36 | ✓ 顶部前置 |
| TBL+TBR | 1.34 | ✓ 顶部后置 |

> **说明**：FC+LFE 声道对因内容特性（人声+低频），置信度通常较低 (~0.45)，但不影响整体检测，因为其他声道对均可正常检测。

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
