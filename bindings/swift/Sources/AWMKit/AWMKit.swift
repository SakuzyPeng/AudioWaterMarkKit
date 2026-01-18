/// AWMKit - Audio Watermark Kit
///
/// 128-bit watermark message codec with:
/// - 8-char base32 tag (7 identity + 1 checksum)
/// - 32-bit UTC timestamp (minutes)
/// - 48-bit HMAC-SHA256 authentication
///
/// # Usage
///
/// ```swift
/// import AWMKit
///
/// // Create key
/// let key = Data("your-32-byte-secret-key-here!!!!".utf8)
///
/// // Create tag
/// let tag = try AWMTag(identity: "SAKUZY")
/// print(tag.value)     // "SAKUZY_2"
/// print(tag.identity)  // "SAKUZY"
///
/// // Encode message
/// let msg = try AWMMessage.encode(tag: tag, key: key)
/// print(msg.hexString) // 32 hex chars
///
/// // Decode message
/// let result = try AWMMessage.decode(msg, key: key)
/// print(result.identity)  // "SAKUZY"
/// print(result.date)      // 2026-01-18 ...
/// ```

@_exported import CAWMKit

// Re-export all public types
public typealias Tag = AWMTag
public typealias Message = AWMMessage
public typealias MessageResult = AWMMessageResult
public typealias Audio = AWMAudio
public typealias DetectResult = AWMDetectResultSwift
