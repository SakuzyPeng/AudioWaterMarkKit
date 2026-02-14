import SwiftUI
import AWMKit
import UniformTypeIdentifiers

struct DetectRecord: Identifiable, Equatable, Sendable {
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
    let verification: String?
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
        verification: String? = nil,
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
        self.verification = verification
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
    private var progressResetTask: Task<Void, Never>?

    deinit {
        progressResetTask?.cancel()
    }

    // MARK: - 日志

    func log(
        _ title: String,
        detail: String = "",
        isSuccess: Bool = true,
        kind: LogEntry.Kind = .generic,
        isEphemeral: Bool = false,
        relatedRecordId: UUID? = nil
    ) {
        let entry = LogEntry(
            title: title,
            detail: detail,
            isSuccess: isSuccess,
            kind: kind,
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

    func selectFiles(appState: AppState) {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowedContentTypes = []

        if panel.runModal() == .OK, let source = panel.url {
            inputSource = source
            let files = resolveAudioFiles(from: source, appState: appState)
            appendFilesWithDedup(files)
        }
    }

    func clearInputSource() {
        guard inputSource != nil else {
            log(
                localized("输入源为空", "Input source is empty"),
                detail: localized("没有可清空的输入源地址", "No input source path to clear"),
                kind: .generic,
                isEphemeral: true
            )
            return
        }
        inputSource = nil
        log(
            localized("已清空输入源", "Input source cleared"),
            detail: localized("仅清空输入源地址，不影响待处理队列", "Cleared input source path only; queue unchanged"),
            kind: .generic,
            isEphemeral: true
        )
    }

    func processDropProviders(_ providers: [NSItemProvider], appState: AppState) {
        var urls: [URL] = []
        let lock = NSLock()
        let group = DispatchGroup()
        for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
            group.enter()
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
                defer { group.leave() }
                if let data = item as? Data, let url = URL(dataRepresentation: data, relativeTo: nil) {
                    lock.lock()
                    urls.append(url)
                    lock.unlock()
                }
            }
        }
        group.notify(queue: .main) { [weak self] in
            guard let self else { return }
            var resolved: [URL] = []
            var unsupported: [URL] = []
            for url in urls {
                if self.isDirectory(url) {
                    resolved.append(contentsOf: self.resolveAudioFiles(from: url, appState: appState))
                } else if self.isSupportedAudioFile(url, appState: appState) {
                    resolved.append(url)
                } else {
                    unsupported.append(url)
                }
            }
            self.logUnsupportedFiles(unsupported, appState: appState)
            self.appendFilesWithDedup(resolved)
        }
    }

    func dropZoneSubtitle(appState: AppState) -> String {
        let extText = appState.supportedInputExtensionsDisplay()
        if appState.audioMediaCapsKnown {
            return appState.tr(
                "支持 \(extText)，可批量拖入并检测",
                "Supports \(extText), batch drop enabled for detection"
            )
        }
        return appState.tr(
            "支持 \(extText)，当前按默认集合处理（运行时能力未知）",
            "Supports \(extText); using default fallback set while runtime capabilities are unknown"
        )
    }

    private func resolveAudioFiles(from source: URL, appState: AppState) -> [URL] {
        if isDirectory(source) {
            do {
                let items = try FileManager.default.contentsOfDirectory(
                    at: source,
                    includingPropertiesForKeys: [.isDirectoryKey],
                    options: [.skipsHiddenFiles]
                )
                let regularFiles = items.filter { !isDirectory($0) }
                let files = regularFiles.filter { isSupportedAudioFile($0, appState: appState) }
                let unsupported = regularFiles.filter { !isSupportedAudioFile($0, appState: appState) }
                logUnsupportedFiles(unsupported, appState: appState)
                if files.isEmpty {
                    log(
                        localized("目录无可用音频", "No audio files in directory"),
                        detail: directoryNoAudioDetail(appState: appState),
                        isSuccess: false,
                        kind: .directoryNoAudio,
                        isEphemeral: true
                    )
                }
                return files
            } catch {
                log(
                    localized("读取目录失败", "Failed to read directory"),
                    detail: error.localizedDescription,
                    isSuccess: false,
                    kind: .directoryReadFailed
                )
                return []
            }
        }

        guard isSupportedAudioFile(source, appState: appState) else {
            log(
                localized("不支持的输入源", "Unsupported input source"),
                detail: unsupportedInputDetail(appState: appState),
                isSuccess: false,
                kind: .unsupportedInput,
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
            log(
                localized("已去重", "Deduplicated"),
                detail: localized("跳过 \(duplicateCount) 个重复文件", "Skipped \(duplicateCount) duplicate files"),
                kind: .deduplicated,
                isEphemeral: true
            )
        }
    }

    private func isSupportedAudioFile(_ url: URL, appState: AppState) -> Bool {
        guard !isDirectory(url) else { return false }
        return effectiveSupportedAudioExtensions(appState: appState).contains(url.pathExtension.lowercased())
    }

    private func effectiveSupportedAudioExtensions(appState: AppState) -> Set<String> {
        Set(appState.effectiveSupportedInputExtensions().map { $0.lowercased() })
    }

    private func directoryNoAudioDetail(appState: AppState) -> String {
        let extText = appState.supportedInputExtensionsDisplay()
        if appState.audioMediaCapsKnown {
            return appState.tr(
                "当前目录未找到 \(extText) 文件",
                "No \(extText) files found in this directory"
            )
        }
        return appState.tr(
            "当前目录未找到 \(extText) 文件（按默认支持集合处理）",
            "No \(extText) files found in this directory (fallback set applied)"
        )
    }

    private func unsupportedInputDetail(appState: AppState) -> String {
        let extText = appState.supportedInputExtensionsDisplay()
        if appState.audioMediaCapsKnown {
            return appState.tr(
                "请选择 \(extText) 文件或包含这些文件的目录",
                "Select a \(extText) file or a directory containing these files"
            )
        }
        return appState.tr(
            "请选择 \(extText) 文件或包含这些文件的目录。当前按默认集合处理，运行时缺少 demuxer 时仍可能失败",
            "Select a \(extText) file or a directory containing these files. Using fallback set now; execution can still fail if demuxers are missing"
        )
    }

    private func logUnsupportedFiles(_ files: [URL], appState: AppState) {
        var seen = Set<String>()
        let unique = files.map(Self.normalizedPathKey).filter { seen.insert($0).inserted }
        guard !unique.isEmpty else { return }

        let preview = unique
            .prefix(3)
            .compactMap { URL(fileURLWithPath: $0).lastPathComponent }
            .joined(separator: ", ")
        let remain = max(unique.count - 3, 0)
        let detail: String
        if remain == 0 {
            detail = appState.tr(
                "已跳过 \(unique.count) 个不支持文件：\(preview)",
                "Skipped \(unique.count) unsupported file(s): \(preview)"
            )
        } else {
            detail = appState.tr(
                "已跳过 \(unique.count) 个不支持文件：\(preview) 等 \(remain) 个",
                "Skipped \(unique.count) unsupported file(s): \(preview) and \(remain) more"
            )
        }

        log(
            localized("已跳过不支持文件", "Skipped unsupported files"),
            detail: detail,
            isSuccess: false,
            kind: .unsupportedInput,
            isEphemeral: true
        )
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
            log(
                localized("队列为空", "Queue is empty"),
                detail: localized("没有可移除的文件", "No files to remove"),
                kind: .queueEmpty,
                isEphemeral: true
            )
            return
        }
        let count = selectedFiles.count
        selectedFiles.removeAll()
        log(
            localized("已清空队列", "Queue cleared"),
            detail: localized("移除了 \(count) 个文件", "Removed \(count) files"),
            kind: .queueCleared
        )
        flash(\.isClearQueueSuccess)
    }

    func clearLogs() {
        guard !logs.isEmpty else {
            log(
                localized("日志为空", "Logs are empty"),
                detail: localized("没有可清空的日志", "No logs to clear"),
                kind: .logsEmpty,
                isEphemeral: true
            )
            return
        }
        let count = logs.count
        logs.removeAll()
        detectRecords.removeAll()
        totalDetected = 0
        totalFound = 0
        log(
            localized("已清空日志", "Logs cleared"),
            detail: localized("移除了 \(count) 条日志记录", "Removed \(count) log entries"),
            kind: .logsCleared,
            isEphemeral: true
        )
        flash(\.isClearLogsSuccess)
    }

    // MARK: - 检测处理

    func detectFiles(audio: AWMAudio?) {
        guard !isProcessing else { return }

        guard !selectedFiles.isEmpty else {
            log(
                localized("队列为空", "Queue is empty"),
                detail: localized("请先添加音频文件", "Add audio files first"),
                isSuccess: false,
                kind: .queueEmpty,
                isEphemeral: true
            )
            return
        }

        progressResetTask?.cancel()
        isProcessing = true
        progress = 0
        currentProcessingIndex = 0
        totalDetected = 0
        totalFound = 0

        log(
            localized("开始检测", "Detection started"),
            detail: localized("准备检测 \(selectedFiles.count) 个文件", "Preparing to detect \(selectedFiles.count) files"),
            kind: .processStarted
        )

        Task {
            guard let audio else {
                log(
                    localized("检测失败", "Detection failed"),
                    detail: localized("AudioWmark 未初始化", "AudioWmark is not initialized"),
                    isSuccess: false,
                    kind: .detectFailed
                )
                isProcessing = false
                return
            }
            let key = try? AWMKeyStore.loadActiveKey()
            if key == nil {
                log(
                    localized("未配置密钥", "Key not configured"),
                    detail: localized(
                        "将仅显示未校验结果，且不可用于归属/取证",
                        "Only unverified fields will be shown. Do not use for attribution/forensics"
                    ),
                    isSuccess: false,
                    kind: .detectFailed,
                    isEphemeral: true
                )
            }
            let audioBox = UnsafeAudioBox(audio: audio)

            let initialTotal = selectedFiles.count
            let total = Double(initialTotal)

            for processedCount in 0..<initialTotal {
                guard let fileURL = selectedFiles.first else { break }
                currentProcessingIndex = 0
                let fileName = fileURL.lastPathComponent
                let record = await Self.performDetectStep(audio: audioBox, fileURL: fileURL, key: key)
                insertDetectRecord(record)
                logDetectionOutcome(fileName: fileName, record: record)
                if record.status == "ok" {
                    totalFound += 1
                }
                totalDetected += 1
                if !selectedFiles.isEmpty {
                    selectedFiles.removeFirst()
                }
                progress = Double(processedCount + 1) / total
                await Task.yield()
            }

            log(
                localized("检测完成", "Detection finished"),
                detail: localized("已检测: \(totalDetected), 发现水印: \(totalFound)", "Detected: \(totalDetected), Found watermark: \(totalFound)"),
                kind: .processFinished
            )

            currentProcessingIndex = -1
            isProcessing = false
            scheduleProgressResetIfNeeded()
        }
    }

    private func logDetectionOutcome(fileName: String, record: DetectRecord) {
        switch record.status {
        case "ok":
            let timeText: String
            if let timestampUTC = record.timestampUTC {
                timeText = Date(timeIntervalSince1970: TimeInterval(timestampUTC)).formatted()
            } else {
                timeText = "-"
            }
            log(
                "\(localized("成功", "Success")): \(fileName)",
                detail: localized(
                    "标签: \(record.identity ?? "-") | 时间: \(timeText) | 克隆: \(record.cloneCheck ?? "-")",
                    "Tag: \(record.identity ?? "-") | Time: \(timeText) | Clone: \(record.cloneCheck ?? "-")"
                ),
                kind: .resultOk,
                relatedRecordId: record.id
            )
        case "not_found":
            log(
                "\(localized("无标记", "Not found")): \(fileName)",
                detail: localized("未检测到水印", "No watermark detected"),
                isSuccess: false,
                kind: .resultNotFound,
                relatedRecordId: record.id
            )
        case "invalid_hmac":
            let warning = localized(
                "UNVERIFIED · 不可用于归属/取证",
                "UNVERIFIED · Do not use for attribution/forensics"
            )
            let reason = record.error ?? "unknown"
            log(
                "\(localized("失败", "Failed")): \(fileName)",
                detail: localized(
                    "HMAC 校验失败: \(reason) · \(warning)",
                    "HMAC verification failed: \(reason) · \(warning)"
                ),
                isSuccess: false,
                kind: .resultInvalidHmac,
                relatedRecordId: record.id
            )
        default:
            log(
                "\(localized("失败", "Failed")): \(fileName)",
                detail: record.error ?? localized("未知错误", "Unknown error"),
                isSuccess: false,
                kind: .resultError,
                relatedRecordId: record.id
            )
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
        inputSource?.path(percentEncoded: false) ?? localized("尚未选择输入源", "No input source selected")
    }

    func fileStatusText(for url: URL, at index: Int) -> (text: String, isActive: Bool) {
        let fileName = url.lastPathComponent
        if let entry = logs.first(where: { $0.title.hasSuffix(fileName) && !$0.isEphemeral }) {
            let status: String
            switch entry.kind {
            case .resultOk:
                status = localized("已检测", "Detected")
            case .resultNotFound:
                status = localized("无标记", "Not found")
            case .resultInvalidHmac, .resultError:
                status = localized("失败", "Failed")
            default:
                status = entry.isSuccess
                    ? localized("已检测", "Detected")
                    : localized("无标记", "Not found")
            }
            return (status, false)
        } else if isProcessing && index == currentProcessingIndex {
            return (localized("检测中", "Detecting"), true)
        } else if isProcessing {
            return (localized("等待中", "Waiting"), false)
        } else {
            return (localized("就绪", "Ready"), false)
        }
    }

    private func localized(_ zh: String, _ en: String) -> String {
        let selected = (try? AWMUILanguageStore.get()) ?? .zhCN
        return selected == .enUS ? en : zh
    }
}

private struct UnsafeAudioBox: @unchecked Sendable {
    let audio: AWMAudio
}

private extension DetectViewModel {
    nonisolated static func performDetectStep(
        audio: UnsafeAudioBox,
        fileURL: URL,
        key: Data?
    ) async -> DetectRecord {
        await Task.detached(priority: .userInitiated) {
            let filePath = fileURL.path(percentEncoded: false)

            do {
                let multichannel = try audio.audio.detectMultichannel(input: fileURL, layout: nil)
                guard let detectResult = multichannel.best else {
                    return DetectRecord(
                        file: filePath,
                        status: "not_found"
                    )
                }

                let unverifiedDecoded = try? AWMMessage.decodeUnverified(detectResult.rawMessage)
                if let key, let decoded = try? AWMMessage.decode(detectResult.rawMessage, key: key) {
                    var cloneKind = "unavailable"
                    var cloneScore: Double?
                    var cloneMatchSeconds: Float?
                    var cloneReason: String?
                    do {
                        let cloneResult = try audio.audio.cloneCheck(
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

                    return DetectRecord(
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
                }

                let failureDetail: String
                if key == nil {
                    failureDetail = "key_not_configured"
                } else {
                    failureDetail = "hmac_verification_failed"
                }

                return DetectRecord(
                    file: filePath,
                    status: "invalid_hmac",
                    tag: unverifiedDecoded?.tag.value,
                    identity: unverifiedDecoded?.identity,
                    version: unverifiedDecoded?.version,
                    timestampMinutes: unverifiedDecoded?.timestampMinutes,
                    timestampUTC: unverifiedDecoded?.timestampUTC,
                    keySlot: unverifiedDecoded?.keySlot,
                    pattern: detectResult.pattern,
                    detectScore: detectResult.detectScore,
                    bitErrors: detectResult.bitErrors,
                    matchFound: detectResult.found,
                    error: failureDetail,
                    verification: "unverified"
                )
            } catch AWMError.noWatermarkFound {
                return DetectRecord(
                    file: filePath,
                    status: "not_found"
                )
            } catch {
                return DetectRecord(
                    file: filePath,
                    status: "error",
                    error: error.localizedDescription
                )
            }
        }.value
    }
}
