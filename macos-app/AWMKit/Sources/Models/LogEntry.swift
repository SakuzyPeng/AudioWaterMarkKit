import Foundation

/// 统一日志条目模型，用于嵌入/检测事件日志展示
struct LogEntry: Identifiable, Equatable {
    let id = UUID()
    let title: String
    let detail: String
    let timestamp: Date
    let isSuccess: Bool
    var isEphemeral: Bool

    init(title: String, detail: String = "", isSuccess: Bool = true, isEphemeral: Bool = false) {
        self.title = title
        self.detail = detail
        self.timestamp = Date()
        self.isSuccess = isSuccess
        self.isEphemeral = isEphemeral
    }

    static func == (lhs: LogEntry, rhs: LogEntry) -> Bool {
        lhs.id == rhs.id
    }
}
