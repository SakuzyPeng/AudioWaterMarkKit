# AWMKit Swift Package

Swift bindings for AWMKit - Audio Watermark Kit.

## 功能

- **Tag**: 8 字符身份标识（7 身份 + 1 校验位）
- **Message**: 128-bit 消息编解码 + HMAC 验证
- **Audio**: 音频水印嵌入/检测（封装 audiowmark）
- **Keychain**: macOS 钥匙串密钥管理

## 安装

### 1. 编译 Rust 库

```bash
cd /path/to/awmkit
cargo build --features ffi --release
```

### 2. 添加 Swift Package

```swift
// Package.swift
dependencies: [
    .package(path: "/path/to/awmkit/bindings/swift")
]
```

或在 Xcode 中：File → Add Package Dependencies → Add Local...

### 3. 安装 audiowmark（音频操作需要）

```bash
# 参考主仓库 vendor/ 目录的预编译版本
# 或自行编译安装
```

## 快速开始

```swift
import AWMKit

// 1. 密钥管理
let keychain = AWMKeychain()
let key = try keychain.generateAndSaveKey()  // 首次：生成并保存
// let key = try AWMKeychain.require()       // 后续：直接读取

// 2. 创建 Tag
let tag = try AWMTag(identity: "SAKUZY")
print(tag.value)     // "SAKUZY_2"
print(tag.identity)  // "SAKUZY"

// 3. 嵌入水印
let audio = try AWMAudio()
try audio.embed(
    input: URL(fileURLWithPath: "input.wav"),
    output: URL(fileURLWithPath: "output.wav"),
    tag: tag,
    key: key
)

// 4. 检测验证
if let result = try audio.detectAndDecode(
    input: URL(fileURLWithPath: "output.wav"),
    key: key
) {
    print("Identity: \(result.identity)")
    print("Time: \(result.date)")
}
```

## API 参考

### AWMTag

```swift
// 创建
let tag = try AWMTag(identity: "SAKUZY")   // 自动补齐 + 校验位
let tag = try AWMTag(tag: "SAKUZY_2")      // 解析并验证

// 属性
tag.value      // String: 完整 8 字符
tag.identity   // String: 身份部分 (1-7 字符)
tag.isValid    // Bool: 校验位是否正确

// Codable 支持
let data = try JSONEncoder().encode(tag)
let tag = try JSONDecoder().decode(AWMTag.self, from: data)
```

### AWMMessage

```swift
// 编码
let msg = try AWMMessage.encode(tag: tag, key: key)
let msg = try AWMMessage.encode(tag: tag, key: key, timestampMinutes: 12345)

// 解码
let result = try AWMMessage.decode(msg, key: key)
result.version          // UInt8
result.timestampUTC     // UInt64 (秒)
result.timestampMinutes // UInt32 (分钟)
result.tag              // AWMTag
result.identity         // String
result.date             // Date

// 仅验证
let valid = AWMMessage.verify(msg, key: key)  // Bool
```

### AWMAudio

```swift
// 创建
let audio = try AWMAudio()                              // 自动搜索
let audio = try AWMAudio(binaryPath: "/path/to/bin")   // 指定路径

// 配置
audio.setStrength(10)                    // 强度 1-30
audio.setKeyFile("/path/to/key")         // audiowmark 密钥文件
audio.isAvailable                        // Bool

// 嵌入
try audio.embed(input: url, output: url, message: data)
try audio.embed(input: url, output: url, tag: tag, key: key)

// 检测
let result = try audio.detect(input: url)  // AWMDetectResultSwift?
result?.found        // Bool
result?.rawMessage   // Data (16 bytes)
result?.pattern      // String ("all" / "single")
result?.bitErrors    // UInt32

// 检测 + 解码
let decoded = try audio.detectAndDecode(input: url, key: key)  // AWMMessageResult?
```

### AWMKeychain

```swift
// 创建
let keychain = AWMKeychain()  // 默认 service/account
let keychain = AWMKeychain(
    service: "com.myapp.watermark",
    account: "signing-key"
)

// 存储
try keychain.saveKey(data)                    // 保存
try keychain.importKey(from: url)             // 从文件导入
let key = try keychain.generateAndSaveKey()   // 生成随机密钥

// 读取
let key = try keychain.loadKey()      // Data? (不存在返回 nil)
let key = try AWMKeychain.require()   // Data (不存在抛错)

// 其他
keychain.hasKey                       // Bool
try keychain.deleteKey()              // 删除
try keychain.exportKey(to: url)       // 导出备份
```

### 错误处理

```swift
do {
    let result = try AWMMessage.decode(msg, key: key)
} catch AWMError.hmacMismatch {
    print("签名验证失败")
} catch AWMError.checksumMismatch {
    print("Tag 校验位错误（可能是 OCR/手抄错误）")
} catch AWMError.audiowmarkNotFound {
    print("未找到 audiowmark")
} catch AWMError.noWatermarkFound {
    print("未检测到水印")
} catch {
    print("其他错误: \(error)")
}
```

### Hex 工具

```swift
// Data 扩展
let hex = data.hexString           // "0123456789abcdef"
let data = Data(hexString: hex)    // Data?
```

## 钥匙串位置

密钥默认存储在：
- **服务**: `com.awmkit.watermark`
- **账户**: `signing-key`
- **访问级别**: 仅解锁时、仅此设备

可在「钥匙串访问.app」中搜索 "AWMKit" 查看。

## 要求

- macOS 12+ / iOS 15+
- Swift 5.9+
- audiowmark（音频操作需要）

## License

MIT License
