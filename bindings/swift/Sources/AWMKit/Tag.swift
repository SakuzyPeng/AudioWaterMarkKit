import Foundation
import CAWMKit

/// 8-character watermark tag (7 identity chars + 1 checksum)
public struct AWMTag: Equatable, Hashable, CustomStringConvertible {
    /// Raw 8-character tag string
    public let value: String

    /// Create tag from identity (1-7 chars, auto-padded with checksum)
    ///
    /// Example:
    /// ```swift
    /// let tag = try AWMTag(identity: "SAKUZY")
    /// print(tag.value)     // "SAKUZY_2"
    /// print(tag.identity)  // "SAKUZY"
    /// ```
    public init(identity: String) throws {
        var buffer = [CChar](repeating: 0, count: 9)
        let result = identity.withCString { ptr in
            awm_tag_new(ptr, &buffer)
        }

        if result != AWM_SUCCESS.rawValue {
            throw AWMError(code: result)
        }

        self.value = String(cString: buffer)
    }

    /// Parse existing 8-character tag (validates checksum)
    public init(tag: String) throws {
        guard tag.count == 8 else {
            throw AWMError.invalidTag("Tag must be 8 characters, got \(tag.count)")
        }

        let valid = tag.withCString { ptr in
            awm_tag_verify(ptr)
        }

        if !valid {
            throw AWMError.checksumMismatch
        }

        self.value = tag.uppercased()
    }

    /// Identity part (without padding and checksum, 1-7 chars)
    public var identity: String {
        var buffer = [CChar](repeating: 0, count: 8)
        _ = value.withCString { ptr in
            awm_tag_identity(ptr, &buffer)
        }
        return String(cString: buffer)
    }

    /// Verify tag checksum
    public var isValid: Bool {
        value.withCString { ptr in
            awm_tag_verify(ptr)
        }
    }

    public var description: String {
        value
    }
}

extension AWMTag: LosslessStringConvertible {
    public init?(_ description: String) {
        do {
            if description.count <= 7 {
                try self.init(identity: description)
            } else {
                try self.init(tag: description)
            }
        } catch {
            return nil
        }
    }
}

extension AWMTag: Codable {
    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let str = try container.decode(String.self)
        try self.init(tag: str)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(value)
    }
}
