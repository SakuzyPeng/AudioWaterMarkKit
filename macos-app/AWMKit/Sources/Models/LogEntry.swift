import Foundation

/// 统一日志条目模型，用于嵌入/检测事件日志展示
struct LogEntry: Identifiable, Equatable {
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

    var iconName: String {
        if title.hasPrefix("成功:") { return "checkmark.circle.fill" }
        if title.hasPrefix("失败:") { return "xmark.circle.fill" }
        if title.hasPrefix("无标记:") { return "questionmark.circle.fill" }

        switch title {
        case "开始处理", "开始检测":
            return "play.circle.fill"
        case "处理完成", "检测完成":
            return "checkmark.seal.fill"
        case "正在中止处理", "已取消":
            return "stop.circle.fill"
        case "已清空队列":
            return "trash.circle.fill"
        case "队列为空":
            return "tray.fill"
        case "已清空日志":
            return "trash.circle.fill"
        case "日志为空":
            return "doc.text.fill"
        case "已去重":
            return "minus.circle.fill"
        case "目录无可用音频":
            return "folder.fill"
        case "标签未填写":
            return "tag.fill"
        case "读取目录失败", "不支持的输入源", "嵌入失败", "检测失败":
            return "exclamationmark.triangle.fill"
        default:
            return isSuccess ? "checkmark.circle.fill" : "xmark.circle.fill"
        }
    }

    var iconTone: IconTone {
        if title.hasPrefix("成功:") { return .success }
        if title.hasPrefix("失败:") { return .error }
        if title.hasPrefix("无标记:") { return .warning }

        switch title {
        case "处理完成", "检测完成", "已清空队列", "已清空日志":
            return .success
        case "开始处理", "开始检测":
            return .info
        case "队列为空", "日志为空", "已去重", "目录无可用音频", "标签未填写", "不支持的输入源", "正在中止处理", "已取消":
            return .warning
        case "读取目录失败", "嵌入失败", "检测失败":
            return .error
        default:
            return isSuccess ? .success : .error
        }
    }
}
