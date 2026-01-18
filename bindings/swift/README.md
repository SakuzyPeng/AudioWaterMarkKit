# AWMKit Swift Package

Swift bindings for AWMKit - Audio Watermark Kit.

## Requirements

- macOS 12+ / iOS 15+
- Swift 5.9+
- Pre-built `libawmkit.a` or `libawmkit.dylib`

## Installation

### Build Rust Library First

```bash
cd /path/to/awmkit
cargo build --features ffi --release
```

### Add to Xcode Project

1. Add this package directory as a local Swift Package
2. Or copy to your project and add as a dependency

## Usage

```swift
import AWMKit

// Create key (recommend 32 bytes)
let key = Data("your-32-byte-secret-key-here!!!!".utf8)

// Create tag from identity (1-7 chars)
let tag = try AWMTag(identity: "SAKUZY")
print(tag.value)     // "SAKUZY_2" (with checksum)
print(tag.identity)  // "SAKUZY"

// Encode message
let msg = try AWMMessage.encode(tag: tag, key: key)
print(msg.hexString) // 32 hex chars

// Decode message
let result = try AWMMessage.decode(msg, key: key)
print(result.identity)  // "SAKUZY"
print(result.date)      // 2026-01-18 ...
print(result.version)   // 1

// Verify only (without decoding)
let isValid = AWMMessage.verify(msg, key: key)
```

## API

### AWMTag

```swift
// Create from identity (1-7 chars)
let tag = try AWMTag(identity: "SAKUZY")

// Parse existing 8-char tag
let tag = try AWMTag(tag: "SAKUZY_2")

// Properties
tag.value      // Full 8-char tag
tag.identity   // Identity without padding/checksum
tag.isValid    // Verify checksum
```

### AWMMessage

```swift
// Encode
let msg = try AWMMessage.encode(tag: tag, key: key)
let msg = try AWMMessage.encode(tag: tag, key: key, timestampMinutes: 12345)

// Decode
let result = try AWMMessage.decode(msg, key: key)

// Verify only
let valid = AWMMessage.verify(msg, key: key)
```

### AWMMessageResult

```swift
result.version          // UInt8: Protocol version
result.timestampUTC     // UInt64: Unix timestamp (seconds)
result.timestampMinutes // UInt32: Unix minutes (raw)
result.tag              // AWMTag: Decoded tag
result.identity         // String: Convenience for tag.identity
result.date             // Date: Convenience for timestamp
```

## Error Handling

```swift
do {
    let result = try AWMMessage.decode(msg, key: key)
} catch AWMError.hmacMismatch {
    print("Invalid signature")
} catch AWMError.checksumMismatch {
    print("Tag checksum error (OCR/typo?)")
} catch {
    print("Other error: \(error)")
}
```

## License

MIT License
