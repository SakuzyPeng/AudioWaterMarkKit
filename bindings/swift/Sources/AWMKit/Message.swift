import Foundation
import CAWMKit

/// Decoded watermark message result
public struct AWMMessageResult {
    /// Protocol version
    public let version: UInt8

    /// UTC Unix timestamp (seconds)
    public let timestampUTC: UInt64

    /// UTC Unix minutes (raw value)
    public let timestampMinutes: UInt32

    /// Key slot (v1 always 0, v2 is 0-31)
    public let keySlot: UInt8

    /// Decoded tag
    public let tag: AWMTag

    /// Identity string (convenience)
    public var identity: String {
        tag.identity
    }

    /// Timestamp as Date
    public var date: Date {
        Date(timeIntervalSince1970: TimeInterval(timestampUTC))
    }
}

/// Watermark message encoder/decoder
public enum AWMMessage {
    /// Current protocol version
    public static let currentVersion: UInt8 = awm_current_version()

    /// Message length in bytes
    public static let messageLength: Int = Int(awm_message_length())

    /// Encode a watermark message
    ///
    /// - Parameters:
    ///   - version: Protocol version (default: currentVersion)
    ///   - tag: 8-character tag
    ///   - key: HMAC key (recommended: 32 bytes)
    /// - Returns: 16-byte message
    public static func encode(
        version: UInt8 = currentVersion,
        tag: AWMTag,
        key: Data
    ) throws -> Data {
        var output = [UInt8](repeating: 0, count: 16)

        let result = tag.value.withCString { tagPtr in
            key.withUnsafeBytes { keyPtr in
                awm_message_encode(
                    version,
                    tagPtr,
                    keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.count,
                    &output
                )
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        return Data(output)
    }

    /// Encode a watermark message with specific timestamp
    ///
    /// - Parameters:
    ///   - version: Protocol version
    ///   - tag: 8-character tag
    ///   - key: HMAC key
    ///   - timestampMinutes: UTC Unix minutes
    /// - Returns: 16-byte message
    public static func encode(
        version: UInt8 = currentVersion,
        tag: AWMTag,
        key: Data,
        timestampMinutes: UInt32
    ) throws -> Data {
        var output = [UInt8](repeating: 0, count: 16)

        let result = tag.value.withCString { tagPtr in
            key.withUnsafeBytes { keyPtr in
                awm_message_encode_with_timestamp(
                    version,
                    tagPtr,
                    keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.count,
                    timestampMinutes,
                    &output
                )
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        return Data(output)
    }

    /// Decode and verify a watermark message
    ///
    /// - Parameters:
    ///   - data: 16-byte message
    ///   - key: HMAC key
    /// - Returns: Decoded result
    /// - Throws: AWMError.hmacMismatch if verification fails
    public static func decode(
        _ data: Data,
        key: Data
    ) throws -> AWMMessageResult {
        guard data.count == 16 else {
            throw AWMError.invalidMessageLength(data.count)
        }

        var cResult = AWMResult()

        let result = data.withUnsafeBytes { dataPtr in
            key.withUnsafeBytes { keyPtr in
                awm_message_decode(
                    dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.count,
                    &cResult
                )
            }
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        // Convert C strings to Swift strings
        let tagStr = withUnsafePointer(to: cResult.tag) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: 9) { charPtr in
                String(cString: charPtr)
            }
        }

        let tag = try AWMTag(tag: tagStr)

        return AWMMessageResult(
            version: cResult.version,
            timestampUTC: cResult.timestamp_utc,
            timestampMinutes: cResult.timestamp_minutes,
            keySlot: cResult.key_slot,
            tag: tag
        )
    }

    /// Verify message HMAC only (without full decoding)
    ///
    /// - Parameters:
    ///   - data: 16-byte message
    ///   - key: HMAC key
    /// - Returns: true if HMAC is valid
    public static func verify(
        _ data: Data,
        key: Data
    ) -> Bool {
        guard data.count == 16 else {
            return false
        }

        return data.withUnsafeBytes { dataPtr in
            key.withUnsafeBytes { keyPtr in
                awm_message_verify(
                    dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.count
                )
            }
        }
    }
}

// MARK: - Hex Encoding

extension Data {
    /// Convert to hex string
    public var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }

    /// Initialize from hex string
    public init?(hexString: String) {
        let hex = hexString.lowercased()
        guard hex.count % 2 == 0 else { return nil }

        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex

        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            guard let byte = UInt8(hex[index..<nextIndex], radix: 16) else {
                return nil
            }
            data.append(byte)
            index = nextIndex
        }

        self = data
    }
}
