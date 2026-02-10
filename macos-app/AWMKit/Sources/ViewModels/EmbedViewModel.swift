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
    @Published var usernameInput: String = "" {
        didSet {
            updateMappingSuggestions()
        }
    }
    @Published private(set) var allMappings: [EmbedTagMappingOption] = []
    @Published private(set) var mappingSuggestions: [EmbedTagMappingOption] = []
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
    private var progressResetTask: Task<Void, Never>?

    init() {
        refreshTagMappings()
    }

    deinit {
        progressResetTask?.cancel()
    }

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

        refreshTagMappings()
        let normalizedUsername = normalizedUsernameInput
        guard let resolvedTag = resolvedTagValue, !normalizedUsername.isEmpty else {
            log("用户名未填写", detail: "请输入用户名以自动生成 Tag", isSuccess: false, isEphemeral: true)
            return
        }

        progressResetTask?.cancel()
        isProcessing = true
        isCancelling = false
        progress = 0
        currentProcessingIndex = 0

        let settingsStr = "用户: \(normalizedUsername) | Tag: \(resolvedTag) | 强度: \(Int(strength))"
        log("开始处理", detail: "准备处理 \(selectedFiles.count) 个文件 | \(settingsStr)")

        Task {
            guard let audio else {
                log("嵌入失败", detail: "AudioWmark 未初始化", isSuccess: false)
                isProcessing = false
                return
            }
            guard let key = try? AWMKeyStore.loadActiveKey() else {
                log("嵌入失败", detail: "密钥未配置", isSuccess: false)
                isProcessing = false
                return
            }
            let audioBox = UnsafeAudioBox(audio: audio)

            let initialTotal = selectedFiles.count
            let total = Double(initialTotal)
            let suffix = customSuffix.isEmpty ? "_wm" : customSuffix
            var successCount = 0
            var failureCount = 0

            for processedCount in 0..<initialTotal {
                if isCancelling { break }
                guard let fileURL = selectedFiles.first else { break }
                currentProcessingIndex = 0

                do {
                    let baseName = fileURL.deletingPathExtension().lastPathComponent
                    let ext = fileURL.pathExtension
                    let outputDir = outputDirectory ?? fileURL.deletingLastPathComponent()
                    let outputURL = outputDir.appendingPathComponent("\(baseName)\(suffix).\(ext)")
                    let step = try await Self.performEmbedStep(
                        audio: audioBox,
                        fileURL: fileURL,
                        outputURL: outputURL,
                        tagValue: resolvedTag,
                        key: key,
                        strength: UInt8(strength)
                    )
                    if let evidenceError = step.evidenceErrorDescription {
                        log(
                            "证据记录失败",
                            detail: "\(outputURL.lastPathComponent): \(evidenceError)",
                            isSuccess: false,
                            isEphemeral: true
                        )
                    }

                    log("成功: \(fileURL.lastPathComponent)", detail: "→ \(outputURL.lastPathComponent)")
                    successCount += 1
                } catch {
                    log("失败: \(fileURL.lastPathComponent)", detail: error.localizedDescription, isSuccess: false)
                    failureCount += 1
                }
                if !selectedFiles.isEmpty {
                    selectedFiles.removeFirst()
                }
                progress = Double(processedCount + 1) / total
                await Task.yield()
            }

            if isCancelling {
                log("已取消", detail: "已完成 \(successCount + failureCount) / \(initialTotal) 个文件", isSuccess: false)
            } else {
                log("处理完成", detail: "成功: \(successCount), 失败: \(failureCount)")
            }

            if successCount > 0 {
                do {
                    let saveResult = try EmbedTagMappingStore.saveIfAbsent(
                        username: normalizedUsername,
                        tag: resolvedTag
                    )
                    if saveResult == .inserted {
                        refreshTagMappings()
                        log("已保存映射", detail: "\(normalizedUsername) -> \(resolvedTag)")
                    }
                } catch {
                    log("保存映射失败", detail: error.localizedDescription, isSuccess: false, isEphemeral: true)
                }
            }

            currentProcessingIndex = -1
            isProcessing = false
            isCancelling = false
            scheduleProgressResetIfNeeded()
        }
    }

    // MARK: - 标签映射

    func refreshTagMappings() {
        allMappings = EmbedTagMappingStore.loadMappings()
        updateMappingSuggestions()
    }

    func selectMapping(_ option: EmbedTagMappingOption) {
        usernameInput = option.username
    }

    var hasMappingSuggestions: Bool {
        !allMappings.isEmpty
    }

    var previewTagText: String {
        resolvedTagValue ?? "-"
    }

    var matchedMappingHintText: String? {
        guard matchedMappingForInput != nil else { return nil }
        return "已存在映射，自动复用"
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

    private var normalizedUsernameInput: String {
        usernameInput.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var resolvedTagValue: String? {
        let username = normalizedUsernameInput
        guard !username.isEmpty else { return nil }

        if let mapped = matchedMappingForInput {
            return mapped.tag
        }

        return EmbedTagMappingStore.previewTag(username: username)
    }

    private func updateMappingSuggestions() {
        let keyword = normalizedUsernameInput
        guard !allMappings.isEmpty else {
            mappingSuggestions = []
            return
        }

        if keyword.isEmpty {
            mappingSuggestions = allMappings
            return
        }

        mappingSuggestions = allMappings.sorted {
            let lhsRank = mappingRank(for: $0, keyword: keyword)
            let rhsRank = mappingRank(for: $1, keyword: keyword)
            if lhsRank != rhsRank {
                return lhsRank < rhsRank
            }
            return $0.username.localizedCaseInsensitiveCompare($1.username) == .orderedAscending
        }
    }

    private var matchedMappingForInput: EmbedTagMappingOption? {
        let username = normalizedUsernameInput
        guard !username.isEmpty else { return nil }
        return allMappings.first(where: {
            $0.username.compare(username, options: [.caseInsensitive, .diacriticInsensitive]) == .orderedSame
        })
    }

    private func mappingRank(for option: EmbedTagMappingOption, keyword: String) -> Int {
        let user = option.username
        if user.compare(keyword, options: [.caseInsensitive, .diacriticInsensitive]) == .orderedSame {
            return 0
        }
        if user.range(of: keyword, options: [.caseInsensitive, .diacriticInsensitive, .anchored]) != nil {
            return 1
        }
        if user.range(of: keyword, options: [.caseInsensitive, .diacriticInsensitive]) != nil {
            return 2
        }
        return 3
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

private struct UnsafeAudioBox: @unchecked Sendable {
    let audio: AWMAudio
}

private struct EmbedStepOutput: Sendable {
    let evidenceErrorDescription: String?
}

private extension EmbedViewModel {
    nonisolated static func performEmbedStep(
        audio: UnsafeAudioBox,
        fileURL: URL,
        outputURL: URL,
        tagValue: String,
        key: Data,
        strength: UInt8
    ) async throws -> EmbedStepOutput {
        try await Task.detached(priority: .userInitiated) {
            let tag = try AWMTag(tag: tagValue)
            audio.audio.setStrength(strength)
            let rawMessage = try audio.audio.embed(input: fileURL, output: outputURL, tag: tag, key: key)
            do {
                try audio.audio.recordEvidence(file: outputURL, rawMessage: rawMessage, key: key)
                return EmbedStepOutput(evidenceErrorDescription: nil)
            } catch {
                return EmbedStepOutput(evidenceErrorDescription: error.localizedDescription)
            }
        }.value
    }
}

struct EmbedTagMappingOption: Equatable {
    let username: String
    let tag: String
}

private enum EmbedTagMappingStore {
    static func loadMappings() -> [EmbedTagMappingOption] {
        do {
            return try AWMDatabaseStore.listTagMappings(limit: 200).compactMap { entry in
                let username = entry.username.trimmingCharacters(in: .whitespacesAndNewlines)
                let tag = entry.tag.uppercased()
                guard !username.isEmpty, (try? AWMTag(tag: tag)) != nil else {
                    return nil
                }
                return EmbedTagMappingOption(username: username, tag: tag)
            }
        } catch {
            return []
        }
    }

    static func saveIfAbsent(username: String, tag: String) throws -> EmbedTagSaveResult {
        let normalizedUsername = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalizedUsername.isEmpty else { return .existed }

        let normalizedTag = tag.uppercased()
        guard (try? AWMTag(tag: normalizedTag)) != nil else { return .existed }

        let inserted = try AWMDatabaseStore.saveTagIfAbsent(username: normalizedUsername, tag: normalizedTag)
        return inserted ? .inserted : .existed
    }

    static func previewTag(username: String) -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }
        if let existing = try? AWMDatabaseStore.lookupTag(username: normalized) {
            return existing
        }
        return try? AWMDatabaseStore.suggestTag(username: normalized)
    }
}

private enum EmbedTagSaveResult {
    case inserted
    case existed
}
