import SwiftUI
import AWMKit
import UniformTypeIdentifiers

struct DetectRecord: Identifiable, Equatable {
    let id: UUID
    let file: String
    let status: String
    let tag: String?
    let identity: String?
    let version: UInt8?
    let timestampMinutes: UInt32?
    let timestampUTC: UInt64?
    let keySlot: UInt8?
    let pattern: String?
    let detectScore: Float?
    let bitErrors: UInt32?
    let matchFound: Bool?
    let cloneCheck: String?
    let cloneScore: Double?
    let cloneMatchSeconds: Float?
    let cloneReason: String?
    let error: String?
    let timestamp: Date

    init(
        id: UUID = UUID(),
        file: String,
        status: String,
        tag: String? = nil,
        identity: String? = nil,
        version: UInt8? = nil,
        timestampMinutes: UInt32? = nil,
        timestampUTC: UInt64? = nil,
        keySlot: UInt8? = nil,
        pattern: String? = nil,
        detectScore: Float? = nil,
        bitErrors: UInt32? = nil,
        matchFound: Bool? = nil,
        cloneCheck: String? = nil,
        cloneScore: Double? = nil,
        cloneMatchSeconds: Float? = nil,
        cloneReason: String? = nil,
        error: String? = nil,
        timestamp: Date = Date()
    ) {
        self.id = id
        self.file = file
        self.status = status
        self.tag = tag
        self.identity = identity
        self.version = version
        self.timestampMinutes = timestampMinutes
        self.timestampUTC = timestampUTC
        self.keySlot = keySlot
        self.pattern = pattern
        self.detectScore = detectScore
        self.bitErrors = bitErrors
        self.matchFound = matchFound
        self.cloneCheck = cloneCheck
        self.cloneScore = cloneScore
        self.cloneMatchSeconds = cloneMatchSeconds
        self.cloneReason = cloneReason
        self.error = error
        self.timestamp = timestamp
    }
}

@MainActor
class DetectViewModel: ObservableObject {
    // MARK: - 文件队列
    @Published var selectedFiles: [URL] = []
    @Published var inputSource: URL?

    // MARK: - 处理状态
    @Published var isProcessing = false
    @Published var progress: Double = 0
    @Published var currentProcessingIndex: Int = -1

    // MARK: - 日志
    @Published var logs: [LogEntry] = []
    @Published var detectRecords: [DetectRecord] = []

    // MARK: - 统计
    @Published var totalDetected: Int = 0
    @Published var totalFound: Int = 0

    // MARK: - 按钮闪烁
    @Published var isClearQueueSuccess = false
    @Published var isClearLogsSuccess = false

    private let maxLogCount = 200
    private let supportedAudioExtensions: Set<String> = ["wav", "flac"]
    private var progressResetTask: Task<Void, Never>?

    deinit {
        progressResetTask?.cancel()
    }

    // MARK: - 日志

    func log(
        _ title: String,
        detail: String = "",
        isSuccess: Bool = true,
        isEphemeral: Bool = false,
        relatedRecordId: UUID? = nil
    ) {
        let entry = LogEntry(
            title: title,
            detail: detail,
            isSuccess: isSuccess,
            isEphemeral: isEphemeral,
            relatedRecordId: relatedRecordId
        )
        logs.insert(entry, at: 0)
        if logs.count > maxLogCount {
            logs.removeLast(logs.count - maxLogCount)
        }
    }

    private func insertDetectRecord(_ record: DetectRecord) {
        detectRecords.insert(record, at: 0)
        if detectRecords.count > maxLogCount {
            detectRecords.removeLast(detectRecords.count - maxLogCount)
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
        detectRecords.removeAll()
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

        progressResetTask?.cancel()
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
            guard let key = try? AWMKeyStore.loadActiveKey() else {
                log("检测失败", detail: "密钥未配置", isSuccess: false)
                isProcessing = false
                return
            }

            let initialTotal = selectedFiles.count
            let total = Double(initialTotal)

            for processedCount in 0..<initialTotal {
                guard let fileURL = selectedFiles.first else { break }
                currentProcessingIndex = 0
                let filePath = fileURL.path(percentEncoded: false)
                let fileName = fileURL.lastPathComponent

                do {
                    if let detectResult = try audio.detect(input: fileURL) {
                        do {
                            let decoded = try AWMMessage.decode(detectResult.rawMessage, key: key)
                            var cloneKind = "unavailable"
                            var cloneScore: Double?
                            var cloneMatchSeconds: Float?
                            var cloneReason: String?
                            do {
                                let cloneResult = try audio.cloneCheck(
                                    input: fileURL,
                                    identity: decoded.identity,
                                    keySlot: decoded.keySlot
                                )
                                cloneKind = cloneResult.kind.rawValue
                                cloneScore = cloneResult.score
                                cloneMatchSeconds = cloneResult.matchSeconds
                                cloneReason = cloneResult.reason
                            } catch {
                                cloneKind = "unavailable"
                                cloneReason = error.localizedDescription
                            }
                            let record = DetectRecord(
                                file: filePath,
                                status: "ok",
                                tag: decoded.tag.value,
                                identity: decoded.identity,
                                version: decoded.version,
                                timestampMinutes: decoded.timestampMinutes,
                                timestampUTC: decoded.timestampUTC,
                                keySlot: decoded.keySlot,
                                pattern: detectResult.pattern,
                                detectScore: detectResult.detectScore,
                                bitErrors: detectResult.bitErrors,
                                matchFound: detectResult.found,
                                cloneCheck: cloneKind,
                                cloneScore: cloneScore,
                                cloneMatchSeconds: cloneMatchSeconds,
                                cloneReason: cloneReason
                            )
                            insertDetectRecord(record)
                            log(
                                "成功: \(fileName)",
                                detail: "标签: \(decoded.identity) | 时间: \(decoded.date.formatted()) | 克隆: \(cloneKind)",
                                relatedRecordId: record.id
                            )
                            totalFound += 1
                        } catch {
                            let record = DetectRecord(
                                file: filePath,
                                status: "invalid_hmac",
                                pattern: detectResult.pattern,
                                detectScore: detectResult.detectScore,
                                bitErrors: detectResult.bitErrors,
                                matchFound: detectResult.found,
                                error: error.localizedDescription
                            )
                            insertDetectRecord(record)
                            log(
                                "失败: \(fileName)",
                                detail: "HMAC 校验失败: \(error.localizedDescription)",
                                isSuccess: false,
                                relatedRecordId: record.id
                            )
                        }
                    } else {
                        let record = DetectRecord(
                            file: filePath,
                            status: "not_found"
                        )
                        insertDetectRecord(record)
                        log(
                            "无标记: \(fileName)",
                            detail: "未检测到水印",
                            isSuccess: false,
                            relatedRecordId: record.id
                        )
                    }
                } catch {
                    let record = DetectRecord(
                        file: filePath,
                        status: "error",
                        error: error.localizedDescription
                    )
                    insertDetectRecord(record)
                    log(
                        "失败: \(fileName)",
                        detail: error.localizedDescription,
                        isSuccess: false,
                        relatedRecordId: record.id
                    )
                }
                totalDetected += 1
                if !selectedFiles.isEmpty {
                    selectedFiles.removeFirst()
                }
                progress = Double(processedCount + 1) / total
            }

            log("检测完成", detail: "已检测: \(totalDetected), 发现水印: \(totalFound)")

            currentProcessingIndex = -1
            isProcessing = false
            scheduleProgressResetIfNeeded()
        }
    }

    private func scheduleProgressResetIfNeeded() {
        guard progress >= 1 else { return }

        progressResetTask?.cancel()
        progressResetTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(3))
            guard !Task.isCancelled else { return }
            await MainActor.run {
                guard let self else { return }
                withAnimation(.easeOut(duration: 0.2)) {
                    self.progress = 0
                }
            }
        }
    }

    // MARK: - 计算属性

    var inputSourceText: String {
        inputSource?.path(percentEncoded: false) ?? "尚未选择输入源"
    }

    func fileStatusText(for url: URL, at index: Int) -> (text: String, isActive: Bool) {
        let fileName = url.lastPathComponent
        if let entry = logs.first(where: { $0.title.hasSuffix(fileName) && !$0.isEphemeral }) {
            let status: String
            if entry.title.hasPrefix("成功:") {
                status = "已检测"
            } else if entry.title.hasPrefix("无标记:") {
                status = "无标记"
            } else if entry.title.hasPrefix("失败:") {
                status = "失败"
            } else {
                status = entry.isSuccess ? "已检测" : "无标记"
            }
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
