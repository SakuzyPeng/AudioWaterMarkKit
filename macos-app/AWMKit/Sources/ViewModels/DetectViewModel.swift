import SwiftUI
import AWMKit
import UniformTypeIdentifiers

@MainActor
class DetectViewModel: ObservableObject {
    // MARK: - 文件队列
    @Published var selectedFiles: [URL] = []

    // MARK: - 处理状态
    @Published var isProcessing = false
    @Published var progress: Double = 0
    @Published var currentProcessingIndex: Int = -1

    // MARK: - 日志
    @Published var logs: [LogEntry] = []

    // MARK: - 统计
    @Published var totalDetected: Int = 0
    @Published var totalFound: Int = 0

    // MARK: - 按钮闪烁
    @Published var isClearQueueSuccess = false
    @Published var isClearLogsSuccess = false

    private let maxLogCount = 200

    // MARK: - 日志

    func log(_ title: String, detail: String = "", isSuccess: Bool = true, isEphemeral: Bool = false) {
        let entry = LogEntry(title: title, detail: detail, isSuccess: isSuccess, isEphemeral: isEphemeral)
        logs.insert(entry, at: 0)
        if logs.count > maxLogCount {
            logs.removeLast(logs.count - maxLogCount)
        }
    }

    private func flash(_ keyPath: ReferenceWritableKeyPath<DetectViewModel, Bool>) {
        self[keyPath: keyPath] = true
        Task {
            try? await Task.sleep(for: .milliseconds(300))
            self[keyPath: keyPath] = false
        }
    }

    // MARK: - 文件选择

    func selectFiles() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowedContentTypes = [.audio]

        if panel.runModal() == .OK {
            selectedFiles.append(contentsOf: panel.urls)
        }
    }

    func processDropProviders(_ providers: [NSItemProvider]) {
        var urls: [URL] = []
        let group = DispatchGroup()
        for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
            group.enter()
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
                defer { group.leave() }
                if let data = item as? Data, let url = URL(dataRepresentation: data, relativeTo: nil) {
                    let ext = url.pathExtension.lowercased()
                    if ext == "wav" || ext == "flac" {
                        urls.append(url)
                    }
                }
            }
        }
        group.notify(queue: .main) { [weak self] in
            guard let self else { return }
            self.selectedFiles.append(contentsOf: urls)
        }
    }

    // MARK: - 清空操作

    func clearQueue() {
        guard !selectedFiles.isEmpty else {
            log("队列为空", detail: "没有可移除的文件", isEphemeral: true)
            return
        }
        let count = selectedFiles.count
        selectedFiles.removeAll()
        log("已清空队列", detail: "移除了 \(count) 个文件")
        flash(\.isClearQueueSuccess)
    }

    func clearLogs() {
        guard !logs.isEmpty else {
            log("日志为空", detail: "没有可清空的日志", isEphemeral: true)
            return
        }
        let count = logs.count
        logs.removeAll()
        totalDetected = 0
        totalFound = 0
        log("已清空日志", detail: "移除了 \(count) 条日志记录", isEphemeral: true)
        flash(\.isClearLogsSuccess)
    }

    // MARK: - 检测处理

    func detectFiles(audio: AWMAudio?) {
        guard !isProcessing else { return }

        guard !selectedFiles.isEmpty else {
            log("队列为空", detail: "请先添加音频文件", isSuccess: false, isEphemeral: true)
            return
        }

        isProcessing = true
        progress = 0
        currentProcessingIndex = 0
        totalDetected = 0
        totalFound = 0

        log("开始检测", detail: "准备检测 \(selectedFiles.count) 个文件")

        Task {
            guard let audio else {
                log("检测失败", detail: "AudioWmark 未初始化", isSuccess: false)
                isProcessing = false
                return
            }
            guard let key = try? AWMKeychain.require() else {
                log("检测失败", detail: "密钥未配置", isSuccess: false)
                isProcessing = false
                return
            }

            let total = Double(selectedFiles.count)

            for (index, fileURL) in selectedFiles.enumerated() {
                currentProcessingIndex = index

                do {
                    if let msgResult = try audio.detectAndDecode(input: fileURL, key: key) {
                        var detailParts: [String] = []
                        detailParts.append("标签: \(msgResult.identity)")
                        detailParts.append("时间: \(msgResult.date.formatted())")
                        log(
                            "成功: \(fileURL.lastPathComponent)",
                            detail: detailParts.joined(separator: " | ")
                        )
                        totalFound += 1
                    } else {
                        log(
                            "无标记: \(fileURL.lastPathComponent)",
                            detail: "未检测到水印",
                            isSuccess: false
                        )
                    }
                } catch {
                    log(
                        "失败: \(fileURL.lastPathComponent)",
                        detail: error.localizedDescription,
                        isSuccess: false
                    )
                }
                totalDetected += 1
                progress = Double(index + 1) / total
            }

            log("检测完成", detail: "已检测: \(totalDetected), 发现水印: \(totalFound)")

            currentProcessingIndex = -1
            isProcessing = false
        }
    }

    // MARK: - 计算属性

    var inputSummaryText: String {
        if selectedFiles.isEmpty { return "尚未添加文件" }
        if selectedFiles.count == 1 { return selectedFiles[0].lastPathComponent }
        return "\(selectedFiles.count) 个音频文件"
    }

    func fileStatusText(for url: URL, at index: Int) -> (text: String, isActive: Bool) {
        let fileName = url.lastPathComponent
        if let entry = logs.first(where: { $0.title.hasSuffix(fileName) && !$0.isEphemeral }) {
            let status = entry.isSuccess ? "已检测" : "无标记"
            return (status, false)
        } else if isProcessing && index == currentProcessingIndex {
            return ("检测中", true)
        } else if isProcessing {
            return ("等待中", false)
        } else {
            return ("就绪", false)
        }
    }
}
