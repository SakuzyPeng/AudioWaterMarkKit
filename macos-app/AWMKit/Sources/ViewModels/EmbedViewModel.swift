import SwiftUI
import AWMKit
import UniformTypeIdentifiers

@MainActor
class EmbedViewModel: ObservableObject {
    // MARK: - 文件队列
    @Published var selectedFiles: [URL] = []
    @Published var inputSource: URL?
    @Published var outputDirectory: URL?

    // MARK: - 嵌入设置
    @Published var tagInput: String = ""
    @Published var strength: Double = 10
    @Published var customSuffix: String = "_wm"

    // MARK: - 处理状态
    @Published var isProcessing = false
    @Published var isCancelling = false
    @Published var progress: Double = 0
    @Published var currentProcessingIndex: Int = -1

    // MARK: - 日志
    @Published var logs: [LogEntry] = []

    // MARK: - 按钮闪烁
    @Published var isClearQueueSuccess = false
    @Published var isClearLogsSuccess = false

    private let maxLogCount = 200
    private let supportedAudioExtensions: Set<String> = ["wav", "flac"]

    // MARK: - 日志

    func log(_ title: String, detail: String = "", isSuccess: Bool = true, isEphemeral: Bool = false) {
        let entry = LogEntry(title: title, detail: detail, isSuccess: isSuccess, isEphemeral: isEphemeral)
        logs.insert(entry, at: 0)
        if logs.count > maxLogCount {
            logs.removeLast(logs.count - maxLogCount)
        }
    }

    private func flash(_ keyPath: ReferenceWritableKeyPath<EmbedViewModel, Bool>) {
        self[keyPath: keyPath] = true
        Task {
            try? await Task.sleep(for: .milliseconds(300))
            self[keyPath: keyPath] = false
        }
    }

    // MARK: - 文件选择

    func selectFiles() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowedContentTypes = []

        if panel.runModal() == .OK, let source = panel.url {
            inputSource = source
            let files = resolveAudioFiles(from: source)
            appendFilesWithDedup(files)
        }
    }

    func selectOutputDirectory() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = false

        if panel.runModal() == .OK {
            outputDirectory = panel.url
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
            self.appendFilesWithDedup(urls)
        }
    }

    private func resolveAudioFiles(from source: URL) -> [URL] {
        if isDirectory(source) {
            do {
                let items = try FileManager.default.contentsOfDirectory(
                    at: source,
                    includingPropertiesForKeys: [.isDirectoryKey],
                    options: [.skipsHiddenFiles]
                )
                let files = items.filter { isSupportedAudioFile($0) }
                if files.isEmpty {
                    log(
                        "目录无可用音频",
                        detail: "当前目录未找到 WAV / FLAC 文件",
                        isSuccess: false,
                        isEphemeral: true
                    )
                }
                return files
            } catch {
                log("读取目录失败", detail: error.localizedDescription, isSuccess: false)
                return []
            }
        }

        guard isSupportedAudioFile(source) else {
            log(
                "不支持的输入源",
                detail: "请选择 WAV / FLAC 文件或包含这些文件的目录",
                isSuccess: false,
                isEphemeral: true
            )
            return []
        }
        return [source]
    }

    private func appendFilesWithDedup(_ files: [URL]) {
        guard !files.isEmpty else { return }

        var existing = Set(selectedFiles.map(Self.normalizedPathKey))
        var deduped: [URL] = []
        var duplicateCount = 0

        for file in files {
            let key = Self.normalizedPathKey(file)
            if existing.insert(key).inserted {
                deduped.append(file)
            } else {
                duplicateCount += 1
            }
        }

        if !deduped.isEmpty {
            selectedFiles.append(contentsOf: deduped)
        }
        if duplicateCount > 0 {
            log("已去重", detail: "跳过 \(duplicateCount) 个重复文件", isEphemeral: true)
        }
    }

    private func isSupportedAudioFile(_ url: URL) -> Bool {
        guard !isDirectory(url) else { return false }
        return supportedAudioExtensions.contains(url.pathExtension.lowercased())
    }

    private func isDirectory(_ url: URL) -> Bool {
        if let value = try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory {
            return value
        }
        return url.hasDirectoryPath
    }

    private static func normalizedPathKey(_ url: URL) -> String {
        url.standardizedFileURL.path(percentEncoded: false)
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
        log("已清空日志", detail: "移除了 \(count) 条日志记录", isEphemeral: true)
        flash(\.isClearLogsSuccess)
    }

    // MARK: - 嵌入处理

    func embedFiles(audio: AWMAudio?) {
        if isProcessing {
            isCancelling = true
            log("正在中止处理", detail: "等待当前文件完成...", isSuccess: false)
            return
        }

        guard !selectedFiles.isEmpty else {
            log("队列为空", detail: "请先添加音频文件", isSuccess: false, isEphemeral: true)
            return
        }

        guard !tagInput.isEmpty else {
            log("标签未填写", detail: "请输入 7 字符身份标签", isSuccess: false, isEphemeral: true)
            return
        }

        isProcessing = true
        isCancelling = false
        progress = 0
        currentProcessingIndex = 0

        let settingsStr = "Tag: \(tagInput) | 强度: \(Int(strength))"
        log("开始处理", detail: "准备处理 \(selectedFiles.count) 个文件 | \(settingsStr)")

        Task {
            guard let audio else {
                log("嵌入失败", detail: "AudioWmark 未初始化", isSuccess: false)
                isProcessing = false
                return
            }
            guard let key = try? AWMKeychain.require() else {
                log("嵌入失败", detail: "密钥未配置", isSuccess: false)
                isProcessing = false
                return
            }

            let total = Double(selectedFiles.count)
            let suffix = customSuffix.isEmpty ? "_wm" : customSuffix
            var successCount = 0
            var failureCount = 0

            for (index, fileURL) in selectedFiles.enumerated() {
                if isCancelling { break }
                currentProcessingIndex = index

                do {
                    let tag = try AWMTag(identity: tagInput)
                    let baseName = fileURL.deletingPathExtension().lastPathComponent
                    let ext = fileURL.pathExtension
                    let outputDir = outputDirectory ?? fileURL.deletingLastPathComponent()
                    let outputURL = outputDir.appendingPathComponent("\(baseName)\(suffix).\(ext)")

                    audio.setStrength(UInt8(strength))
                    _ = try audio.embed(input: fileURL, output: outputURL, tag: tag, key: key)

                    log("成功: \(fileURL.lastPathComponent)", detail: "→ \(outputURL.lastPathComponent)")
                    successCount += 1
                } catch {
                    log("失败: \(fileURL.lastPathComponent)", detail: error.localizedDescription, isSuccess: false)
                    failureCount += 1
                }
                progress = Double(index + 1) / total
            }

            if isCancelling {
                log("已取消", detail: "已完成 \(successCount + failureCount) / \(selectedFiles.count) 个文件", isSuccess: false)
            } else {
                log("处理完成", detail: "成功: \(successCount), 失败: \(failureCount)")
            }

            currentProcessingIndex = -1
            isProcessing = false
            isCancelling = false
        }
    }

    // MARK: - 计算属性

    var inputSourceText: String {
        inputSource?.path(percentEncoded: false) ?? "尚未选择输入源"
    }

    var outputDirectoryText: String {
        outputDirectory?.path(percentEncoded: false) ?? "默认写回各文件所在目录"
    }

    func fileStatusText(for url: URL, at index: Int) -> (text: String, isActive: Bool) {
        let fileName = url.lastPathComponent
        if let entry = logs.first(where: { $0.title.hasSuffix(fileName) && !$0.isEphemeral }) {
            let status = entry.isSuccess ? "完成" : "失败"
            return (status, false)
        } else if isProcessing && index == currentProcessingIndex {
            return ("处理中", true)
        } else if isProcessing {
            return ("等待中", false)
        } else {
            return ("就绪", false)
        }
    }
}
