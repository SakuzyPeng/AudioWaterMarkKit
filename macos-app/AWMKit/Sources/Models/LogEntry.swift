import Foundation

/// 统一日志条目模型，用于嵌入/检测事件日志展示
struct LogEntry: Identifiable, Equatable {
    enum Kind {
        case generic
        case processStarted
        case processFinished
        case processCancelling
        case processCancelled
        case queueCleared
        case logsCleared
        case queueEmpty
        case logsEmpty
        case deduplicated
        case directoryNoAudio
        case directoryReadFailed
        case unsupportedInput
        case usernameMissing
        case embedFailed
        case detectFailed
        case resultOk
        case resultNotFound
        case resultInvalidHmac
        case resultError
        case mappingSaved
        case evidenceWarning
    }

    enum IconTone {
        case success
        case info
        case warning
        case error
    }

    let id = UUID()
    let title: String
    let detail: String
    let timestamp: Date
    let isSuccess: Bool
    let kind: Kind
    var isEphemeral: Bool
    let relatedRecordId: UUID?

    init(
        title: String,
        detail: String = "",
        isSuccess: Bool = true,
        kind: Kind = .generic,
        isEphemeral: Bool = false,
        relatedRecordId: UUID? = nil
    ) {
        self.title = title
        self.detail = detail
        self.timestamp = Date()
        self.isSuccess = isSuccess
        self.kind = kind
        self.isEphemeral = isEphemeral
        self.relatedRecordId = relatedRecordId
    }

    static func == (lhs: LogEntry, rhs: LogEntry) -> Bool {
        lhs.id == rhs.id
    }

    var iconName: String {
        switch kind {
        case .processStarted:
            return "play.circle.fill"
        case .processFinished:
            return "checkmark.seal.fill"
        case .processCancelling, .processCancelled:
            return "stop.circle.fill"
        case .queueCleared:
            return "trash.circle.fill"
        case .queueEmpty:
            return "tray.fill"
        case .logsCleared:
            return "trash.circle.fill"
        case .logsEmpty:
            return "doc.text.fill"
        case .deduplicated:
            return "minus.circle.fill"
        case .directoryNoAudio:
            return "folder.fill"
        case .usernameMissing:
            return "tag.fill"
        case .directoryReadFailed, .unsupportedInput, .embedFailed, .detectFailed:
            return "exclamationmark.triangle.fill"
        case .resultOk:
            return "checkmark.circle.fill"
        case .resultNotFound:
            return "questionmark.circle.fill"
        case .resultInvalidHmac, .resultError:
            return "xmark.circle.fill"
        case .mappingSaved:
            return "tag.circle.fill"
        case .evidenceWarning:
            return "exclamationmark.circle.fill"
        default:
            return isSuccess ? "checkmark.circle.fill" : "xmark.circle.fill"
        }
    }

    var iconTone: IconTone {
        switch kind {
        case .resultOk, .processFinished, .queueCleared, .logsCleared, .mappingSaved:
            return .success
        case .processStarted:
            return .info
        case .queueEmpty, .logsEmpty, .deduplicated, .directoryNoAudio, .usernameMissing, .unsupportedInput, .processCancelling, .processCancelled, .resultNotFound, .evidenceWarning:
            return .warning
        case .directoryReadFailed, .embedFailed, .detectFailed, .resultInvalidHmac, .resultError:
            return .error
        default:
            return isSuccess ? .success : .error
        }
    }
}
