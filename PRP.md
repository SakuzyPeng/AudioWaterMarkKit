# AWMKit - Audio Watermark Kit

## 产品定义

跨语言音频水印消息编解码库，提供 128-bit 自描述、可验证的水印消息格式。

## 核心价值

| 特性 | 说明 |
|------|------|
| 纯离线 | 无需数据库，消息自描述 |
| 可追溯 | 7 字符身份标识 + UTC 时间戳 |
| 防篡改 | 48-bit HMAC 认证 |
| 跨语言 | Rust/C/ObjC/Swift/Python/WASM |

---

## 消息格式规范 v1.0

```
┌──────────┬────────────┬──────────────────┬────────────┐
│ Version  │ Timestamp  │  UserTagPacked   │   HMAC     │
│  1 byte  │  4 bytes   │    5 bytes       │  6 bytes   │
│  8 bit   │   32 bit   │    40 bit        │   48 bit   │
└──────────┴────────────┴──────────────────┴────────────┘
              总计: 16 bytes = 128 bit
```

### 字段定义

| 字段 | 大小 | 格式 | 说明 |
|------|------|------|------|
| Version | 1 byte | `u8` | 协议版本，当前 `0x01` |
| Timestamp | 4 bytes | `u32` big-endian | Unix minutes (UTC) |
| UserTagPacked | 5 bytes | 8×5bit packed | 7 字符身份 + 1 校验位 |
| HMAC | 6 bytes | 前 6 字节 | `HMAC-SHA256(key, M)[0:6]` |

### 字符集 (32 字符, 5 bit/char)

```
ABCDEFGHJKMNPQRSTUVWXYZ23456789_
```
索引 0-31，排除易混淆字符：`O`, `0`, `I`, `1`, `L`

### 校验位算法

```
PRIMES = [3, 5, 7, 11, 13, 17, 19]
check_index = (sum(char_index[i] * PRIMES[i] for i in 0..7)) % 32
check_char = CHARSET[check_index]
```

### HMAC 计算

```
M = Version(1) || Timestamp(4, big-endian) || TagPacked(5)
mac = HMAC-SHA256(key, M)[0:6]
```

---

## 架构设计

```
awmkit/
├── src/
│   ├── lib.rs           # 公共 API 导出
│   ├── message.rs       # 消息编解码核心
│   ├── tag.rs           # Tag 编解码 + 校验
│   ├── charset.rs       # 字符集定义
│   ├── error.rs         # 错误类型
│   ├── ffi.rs           # C ABI 导出
│   └── bin/
│       └── awm.rs       # CLI 工具
├── include/
│   └── awmkit.h         # C 头文件
├── bindings/
│   ├── python/          # PyO3 绑定
│   ├── swift/           # Swift Package
│   └── wasm/            # wasm-bindgen
├── Cargo.toml
└── PRP.md
```

---

## API 设计

### Rust API

```rust
use awmkit::{Message, Tag, Error};

// Tag 操作
let tag = Tag::new("SAKUZY")?;        // → "SAKUZY_X" (自动补齐+校验)
let tag = Tag::parse("SAKUZY_X")?;    // 解析并验证校验位
assert!(tag.verify());
println!("{}", tag.identity());        // → "SAKUZY"

// 消息编码
let key = b"your-32-byte-secret-key-here!!!!";
let msg = Message::encode(1, &tag, key)?;
assert_eq!(msg.len(), 16);

// 消息解码
let result = Message::decode(&msg, key)?;
println!("Version: {}", result.version);
println!("Time: {}", result.timestamp_utc);
println!("Identity: {}", result.tag.identity());
```

### C FFI

```c
#include <awmkit.h>

// Tag 操作
char tag[9];
int ret = awm_tag_new("SAKUZY", tag);  // → "SAKUZY_X"

bool valid = awm_tag_verify("SAKUZY_X");

// 消息编码
uint8_t msg[16];
ret = awm_message_encode(1, "SAKUZY_X", key, key_len, msg);

// 消息解码
AWMResult result;
ret = awm_message_decode(msg, 16, key, key_len, &result);
printf("Identity: %s\n", result.identity);
printf("Timestamp: %u\n", result.timestamp_utc);
```

### Swift (via C FFI)

```swift
import AWMKit

let tag = AWMTag(identity: "SAKUZY")  // → "SAKUZY_X"
let msg = AWMMessage.encode(version: 1, tag: tag, key: key)
let result = AWMMessage.decode(msg, key: key)
print("Identity: \(result.identity)")
```

### Python (via PyO3)

```python
from awmkit import Tag, Message

tag = Tag.new("SAKUZY")  # → "SAKUZY_X"
msg = Message.encode(1, tag, key)
result = Message.decode(msg, key)
print(f"Identity: {result.identity}")
```

---

## CLI 设计

```bash
# 生成 Tag
awm tag SAKUZY
# → SAKUZY_X

# 验证 Tag
awm tag --verify SAKUZY_X
# → Valid: identity=SAKUZY

# 编码消息
awm encode --tag SAKUZY_X --key-file /path/to/key
# → 01abcdef... (32 hex chars)

# 解码消息
awm decode --hex 01abcdef... --key-file /path/to/key
# → Version: 1
# → Timestamp: 2026-01-18 15:30:00 UTC
# → Identity: SAKUZY
# → Tag: SAKUZY_X
# → Status: Valid
```

---

## 实现计划

### Phase 1: 核心库
- [ ] `charset.rs` - 字符集常量
- [ ] `tag.rs` - Tag 编解码 + 校验位
- [ ] `message.rs` - 消息编解码 + HMAC
- [ ] `error.rs` - 错误类型
- [ ] `lib.rs` - 公共 API
- [ ] 单元测试

### Phase 2: C FFI
- [ ] `ffi.rs` - C ABI 函数导出
- [ ] `include/awmkit.h` - C 头文件
- [ ] 编译验证 (cdylib/staticlib)

### Phase 3: CLI
- [ ] `bin/awm.rs` - CLI 实现
- [ ] tag/encode/decode 子命令

### Phase 4: 跨语言绑定
- [ ] Swift Package (C FFI 封装)
- [ ] Python (PyO3)
- [ ] WASM (wasm-bindgen)

---

## 测试向量

```
# 测试用例 1
Identity: "SAKUZY"
Tag: "SAKUZY_?" (校验位待计算)
Key: 0x00112233...ff (32 bytes)
Timestamp: 29049600 (2026-01-18 00:00 UTC, minutes)
Version: 1
Expected Message: ??? (待实现后生成)
```

---

## 安全考虑

1. **密钥管理**: 库不存储密钥，由调用方负责
2. **时间源**: 调用方需确保 UTC 时间准确
3. **48-bit HMAC**: 对离线场景足够（在线攻击成本高）
4. **校验位**: 防止 OCR/手抄错误，非安全功能

---

## 依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| hmac | 0.12 | HMAC 计算 |
| sha2 | 0.10 | SHA-256 |
| thiserror | 2 | 错误处理 |
| clap | 4 | CLI (可选) |

---

## 许可证

MIT License
