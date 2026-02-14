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
    case audiowmarkNotFound
    case audiowmarkExec(String)
    case noWatermarkFound
    case keyAlreadyExists
    case invalidOutputFormat(String)
    case admUnsupported(String)
    case admPreserveFailed(String)
    case admPcmFormatUnsupported(String)
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
        case AWM_ERROR_AUDIOWMARK_NOT_FOUND.rawValue:
            self = .audiowmarkNotFound
        case AWM_ERROR_AUDIOWMARK_EXEC.rawValue:
            self = .audiowmarkExec("audiowmark process failed")
        case AWM_ERROR_NO_WATERMARK_FOUND.rawValue:
            self = .noWatermarkFound
        case AWM_ERROR_KEY_ALREADY_EXISTS.rawValue:
            self = .keyAlreadyExists
        case AWM_ERROR_INVALID_OUTPUT_FORMAT.rawValue:
            self = .invalidOutputFormat("output must be .wav")
        case AWM_ERROR_ADM_UNSUPPORTED.rawValue:
            self = .admUnsupported("ADM/BWF metadata is unsupported for this operation")
        case AWM_ERROR_ADM_PRESERVE_FAILED.rawValue:
            self = .admPreserveFailed("failed to preserve ADM/BWF metadata while embedding")
        case AWM_ERROR_ADM_PCM_FORMAT_UNSUPPORTED.rawValue:
            self = .admPcmFormatUnsupported("unsupported ADM/BWF PCM format (only 16/24/32-bit PCM)")
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
        case .audiowmarkNotFound:
            return "audiowmark binary not found"
        case .audiowmarkExec(let msg):
            return "audiowmark execution failed: \(msg)"
        case .noWatermarkFound:
            return "No watermark found in audio"
        case .keyAlreadyExists:
            return "Key already exists in slot"
        case .invalidOutputFormat(let message):
            return "Invalid output format: \(message)"
        case .admUnsupported(let message):
            return "ADM/BWF unsupported: \(message)"
        case .admPreserveFailed(let message):
            return "ADM/BWF preserve failed: \(message)"
        case .admPcmFormatUnsupported(let message):
            return "ADM/BWF PCM format unsupported: \(message)"
        case .unknown(let code):
            return "Unknown error: \(code)"
        }
    }
}
