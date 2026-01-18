import Foundation
import CAWMKit

/// AWMKit errors
public enum AWMError: Error, LocalizedError {
    case invalidTag(String)
    case invalidMessageLength(Int)
    case hmacMismatch
    case checksumMismatch
    case nullPointer
    case invalidUtf8
    case unknown(Int32)

    init(code: Int32) {
        switch code {
        case AWM_ERROR_INVALID_TAG.rawValue:
            self = .invalidTag("Invalid tag format")
        case AWM_ERROR_INVALID_MESSAGE_LENGTH.rawValue:
            self = .invalidMessageLength(0)
        case AWM_ERROR_HMAC_MISMATCH.rawValue:
            self = .hmacMismatch
        case AWM_ERROR_CHECKSUM_MISMATCH.rawValue:
            self = .checksumMismatch
        case AWM_ERROR_NULL_POINTER.rawValue:
            self = .nullPointer
        case AWM_ERROR_INVALID_UTF8.rawValue:
            self = .invalidUtf8
        default:
            self = .unknown(code)
        }
    }

    public var errorDescription: String? {
        switch self {
        case .invalidTag(let msg):
            return "Invalid tag: \(msg)"
        case .invalidMessageLength(let len):
            return "Invalid message length: \(len), expected 16"
        case .hmacMismatch:
            return "HMAC verification failed"
        case .checksumMismatch:
            return "Tag checksum mismatch"
        case .nullPointer:
            return "Null pointer error"
        case .invalidUtf8:
            return "Invalid UTF-8 string"
        case .unknown(let code):
            return "Unknown error: \(code)"
        }
    }
}
