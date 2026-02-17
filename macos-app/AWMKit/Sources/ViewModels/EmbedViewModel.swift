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
    @Published private(set) var skippedWatermarkedFiles: [URL] = []
    @Published private(set) var skipSummaryPromptVersion: Int = 0

    private let maxLogCount = 200
    private var progressResetTask: Task<Void, Never>?

    init() {
        refreshTagMappings()
    }

    deinit {
        progressResetTask?.cancel()
    }

    // MARK: - 日志

    func log(
        _ title: String,
        detail: String = "",
        isSuccess: Bool = true,
        kind: LogEntry.Kind = .generic,
        isEphemeral: Bool = false
    ) {
        let entry = LogEntry(
            title: title,
            detail: detail,
            isSuccess: isSuccess,
            kind: kind,
            isEphemeral: isEphemeral
        )
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

    func selectOutputDirectory() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = false

        if panel.runModal() == .OK {
            outputDirectory = panel.url
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

    func clearOutputDirectory() {
        guard outputDirectory != nil else {
            log(
                localized("输出目录为空", "Output directory is empty"),
                detail: localized("没有可清空的输出目录地址", "No output directory path to clear"),
                kind: .generic,
                isEphemeral: true
            )
            return
        }
        outputDirectory = nil
        log(
            localized("已清空输出目录", "Output directory cleared"),
            detail: localized("已恢复为写回源文件目录", "Reset to write-back source directory"),
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
                "支持 \(extText)，可批量拖入",
                "Supports \(extText), batch drop enabled"
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

        var existing = Set(selectedFiles.map(normalizedPathKey))
        var deduped: [URL] = []
        var duplicateCount = 0

        for file in files {
            let key = normalizedPathKey(file)
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
        let unique = files.map(normalizedPathKey).filter { seen.insert($0).inserted }
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

    private func normalizedOutputExtension(from ext: String) -> String {
        return "wav"
    }

    private func isDirectory(_ url: URL) -> Bool {
        if let value = try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory {
            return value
        }
        return url.hasDirectoryPath
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
        log(
            localized("已清空日志", "Logs cleared"),
            detail: localized("移除了 \(count) 条日志记录", "Removed \(count) log entries"),
            kind: .logsCleared,
            isEphemeral: true
        )
        flash(\.isClearLogsSuccess)
    }

    // MARK: - 嵌入处理

    func embedFiles(audio: AWMAudio?) {
        if isProcessing {
            isCancelling = true
            log(
                localized("正在中止处理", "Stopping processing"),
                detail: localized("等待当前文件完成...", "Waiting for current file to finish..."),
                isSuccess: false,
                kind: .processCancelling
            )
            return
        }
        startEmbedPass(audio: audio)
    }

    private func requestSkipSummaryPrompt() {
        skipSummaryPromptVersion += 1
    }

    private func startEmbedPass(audio: AWMAudio?) {
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

        refreshTagMappings()
        let normalizedUsername = normalizedUsernameInput
        guard let resolvedTag = resolvedTagValue, !normalizedUsername.isEmpty else {
            log(
                localized("用户名未填写", "Username is missing"),
                detail: localized("请输入用户名以自动生成 Tag", "Enter username to generate tag automatically"),
                isSuccess: false,
                kind: .usernameMissing,
                isEphemeral: true
            )
            return
        }

        progressResetTask?.cancel()
        isProcessing = true
        isCancelling = false
        progress = 0
        currentProcessingIndex = 0
        skippedWatermarkedFiles = []

        let settingsStr = localized(
            "用户: \(normalizedUsername) | Tag: \(resolvedTag) | 强度: \(Int(strength))",
            "User: \(normalizedUsername) | Tag: \(resolvedTag) | Strength: \(Int(strength))"
        )
        log(
            localized("开始处理", "Processing started"),
            detail: localized("准备处理 \(selectedFiles.count) 个文件", "Preparing to process \(selectedFiles.count) files") + " | \(settingsStr)",
            kind: .processStarted
        )

        Task {
            guard let audio else {
                log(
                    localized("嵌入失败", "Embed failed"),
                    detail: localized("AudioWmark 未初始化", "AudioWmark is not initialized"),
                    isSuccess: false,
                    kind: .embedFailed
                )
                isProcessing = false
                return
            }
            guard let key = try? AWMKeyStore.loadActiveKey() else {
                log(
                    localized("嵌入失败", "Embed failed"),
                    detail: localized("密钥未配置", "Key not configured"),
                    isSuccess: false,
                    kind: .embedFailed
                )
                isProcessing = false
                return
            }
            let activeKeySlot = (try? AWMKeyStore.activeSlot()) ?? 0
            let audioBox = UnsafeAudioBox(audio: audio)

            let initialQueue = selectedFiles
            let initialTotal = max(initialQueue.count, 1)
            let weightByFile = buildProgressWeights(for: initialQueue)
            let totalWeight = max(weightByFile.values.reduce(0, +), 1)
            let suffix = customSuffix.isEmpty ? "_wm" : customSuffix
            var doneWeight = 0.0
            var state = EmbedLoopState()

            for _ in 0..<initialTotal {
                if isCancelling { break }
                guard let fileURL = selectedFiles.first else { break }
                let fileKey = normalizedPathKey(fileURL)
                guard let queueIndex = selectedFiles.firstIndex(where: { normalizedPathKey($0) == fileKey }) else { continue }
                currentProcessingIndex = queueIndex
                let fileWeight = weightByFile[fileKey] ?? 1
                var fileProgress = 0.0
                let updateFileProgress: (Double) -> Void = { [self] candidate in
                    let clamped = min(max(candidate, 0), 1)
                    guard clamped > fileProgress else { return }
                    fileProgress = clamped
                    self.progress = min(1, (doneWeight + (fileWeight * fileProgress)) / totalWeight)
                }
                updateFileProgress(0.02)

                if await runPrecheckPhase(
                    audioBox: audioBox,
                    fileURL: fileURL,
                    fileKey: fileKey,
                    queueIndex: queueIndex,
                    updateFileProgress: updateFileProgress,
                    state: &state
                ) {
                    doneWeight += fileWeight
                    progress = min(1, doneWeight / totalWeight)
                    await Task.yield()
                    continue
                }

                await runEmbedPhase(
                    audio: audio,
                    audioBox: audioBox,
                    fileURL: fileURL,
                    resolvedTag: resolvedTag,
                    key: key,
                    activeKeySlot: activeKeySlot,
                    suffix: suffix,
                    fileProgress: fileProgress,
                    updateFileProgress: updateFileProgress,
                    state: &state
                )
                if let indexToRemove = selectedFiles.firstIndex(where: { normalizedPathKey($0) == fileKey }) {
                    selectedFiles.remove(at: indexToRemove)
                }
                doneWeight += fileWeight
                progress = min(1, doneWeight / totalWeight)
                await Task.yield()
            }

            if isCancelling {
                log(
                    localized("已取消", "Cancelled"),
                    detail: localized(
                        "已完成 \(state.successCount + state.failureCount) / \(initialTotal) 个文件",
                        "Completed \(state.successCount + state.failureCount) / \(initialTotal) files"
                    ),
                    isSuccess: false,
                    kind: .processCancelled
                )
            } else {
                log(
                    localized("处理完成", "Processing finished"),
                    detail: localized("成功: \(state.successCount), 失败: \(state.failureCount)", "Success: \(state.successCount), Failed: \(state.failureCount)"),
                    kind: .processFinished
                )
            }

            if state.successCount > 0 {
                do {
                    let saveResult = try EmbedTagMappingStore.saveIfAbsent(
                        username: normalizedUsername,
                        tag: resolvedTag
                    )
                    if saveResult == .inserted {
                        refreshTagMappings()
                        log(
                            localized("已保存映射", "Mapping saved"),
                            detail: "\(normalizedUsername) -> \(resolvedTag)",
                            kind: .mappingSaved
                        )
                    }
                } catch {
                    log(
                        localized("保存映射失败", "Failed to save mapping"),
                        detail: error.localizedDescription,
                        isSuccess: false,
                        kind: .embedFailed,
                        isEphemeral: true
                    )
                }
            }

            currentProcessingIndex = -1
            isProcessing = false
            isCancelling = false
            scheduleProgressResetIfNeeded()

            if !isCancelling, !state.skippedFiles.isEmpty {
                skippedWatermarkedFiles = state.skippedFiles
                log(
                    localized("已跳过含水印文件", "Skipped watermarked files"),
                    detail: localized(
                        "共跳过 \(state.skippedFiles.count) 个已含水印文件",
                        "Skipped \(state.skippedFiles.count) already-watermarked files"
                    ),
                    isSuccess: false,
                    kind: .resultNotFound
                )
                requestSkipSummaryPrompt()
            }
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
        return localized("已存在映射，自动复用", "Existing mapping found, auto reused")
    }

    var skipSummaryCount: Int {
        skippedWatermarkedFiles.count
    }

    var skipSummaryMessage: String {
        let preview = skippedWatermarkedFiles.prefix(3).map(\.lastPathComponent).joined(separator: "、")
        if skippedWatermarkedFiles.count <= 3 {
            return localized(
                "已跳过 \(skippedWatermarkedFiles.count) 个已含水印文件：\(preview)",
                "Skipped \(skippedWatermarkedFiles.count) already-watermarked files: \(preview)"
            )
        }
        let remain = skippedWatermarkedFiles.count - 3
        return localized(
            "已跳过 \(skippedWatermarkedFiles.count) 个已含水印文件：\(preview) 等 \(remain) 个",
            "Skipped \(skippedWatermarkedFiles.count) already-watermarked files: \(preview) and \(remain) more"
        )
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
        inputSource?.path(percentEncoded: false) ?? localized("尚未选择输入源", "No input source selected")
    }

    var outputDirectoryText: String {
        outputDirectory?.path(percentEncoded: false) ?? localized("默认写回各文件所在目录", "Default: write back to source directory")
    }

    func fileStatusText(for url: URL, at index: Int) -> (text: String, isActive: Bool) {
        let fileName = url.lastPathComponent
        if let entry = logs.first(where: { $0.title.hasSuffix(fileName) && !$0.isEphemeral }) {
            let status = entry.isSuccess
                ? localized("完成", "Done")
                : localized("失败", "Failed")
            return (status, false)
        } else if isProcessing && index == currentProcessingIndex {
            return (localized("处理中", "Processing"), true)
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

private struct EmbedStepOutput: Sendable {
    let evidenceErrorDescription: String?
    let snrDb: Double?
    let snrStatus: String?
    let snrDetail: String?
}

private struct EmbedLoopState {
    var successCount: Int = 0
    var failureCount: Int = 0
    var skippedFiles: [URL] = []
    var skippedKeys: Set<String> = []
}

private extension EmbedViewModel {
    nonisolated static func performEmbedStep(
        audio: UnsafeAudioBox,
        fileURL: URL,
        outputURL: URL,
        tagValue: String,
        key: Data,
        keySlot: UInt8,
        strength: UInt8
    ) async throws -> EmbedStepOutput {
        try await Task.detached(priority: .userInitiated) {
            let tag = try AWMTag(tag: tagValue)
            audio.audio.setStrength(strength)
            let rawMessage = try AWMMessage.encode(tag: tag, key: key, keySlot: keySlot)
            try audio.audio.embedMultichannel(input: fileURL, output: outputURL, message: rawMessage, layout: nil)
            do {
                let snr = try audio.audio.recordEmbedEvidence(
                    input: fileURL,
                    output: outputURL,
                    rawMessage: rawMessage,
                    key: key,
                    isForcedEmbed: false
                )
                return EmbedStepOutput(
                    evidenceErrorDescription: nil,
                    snrDb: snr.snrDb,
                    snrStatus: snr.snrStatus,
                    snrDetail: snr.snrDetail
                )
            } catch {
                return EmbedStepOutput(
                    evidenceErrorDescription: error.localizedDescription,
                    snrDb: nil,
                    snrStatus: nil,
                    snrDetail: nil
                )
            }
        }.value
    }

    nonisolated static func performPrecheckStep(
        audio: UnsafeAudioBox,
        fileURL: URL
    ) async throws -> Bool {
        try await Task.detached(priority: .userInitiated) {
            do {
                let detectResult = try audio.audio.detectMultichannel(input: fileURL, layout: nil)
                return detectResult.best != nil
            } catch AWMError.noWatermarkFound {
                return false
            }
        }.value
    }

    /// 预检阶段：检测已有水印。返回 true 表示此文件应跳过（caller 负责更新 doneWeight 并 continue）。
    func runPrecheckPhase(
        audioBox: UnsafeAudioBox,
        fileURL: URL,
        fileKey: String,
        queueIndex: Int,
        updateFileProgress: @escaping (Double) -> Void,
        state: inout EmbedLoopState
    ) async -> Bool {
        do {
            updateFileProgress(0.06)
            let hasWatermark = try await Self.performPrecheckStep(audio: audioBox, fileURL: fileURL)
            updateFileProgress(0.15)
            if hasWatermark {
                if queueIndex < selectedFiles.count {
                    selectedFiles.remove(at: queueIndex)
                }
                if state.skippedKeys.insert(fileKey).inserted {
                    state.skippedFiles.append(fileURL)
                }
                log(
                    localized("检测到已有水印", "Existing watermark detected"),
                    detail: localized(
                        "\(fileURL.lastPathComponent) 已跳过",
                        "\(fileURL.lastPathComponent) skipped"
                    ),
                    isSuccess: false,
                    kind: .resultNotFound
                )
                return true
            }
        } catch let awmError as AWMError {
            if case .admUnsupported = awmError {
                log(
                    localized("预检已跳过", "Precheck skipped"),
                    detail: localized(
                        "ADM/BWF 检测暂不支持，已跳过预检并继续嵌入",
                        "ADM/BWF detect is not supported yet; precheck was skipped and embed continues"
                    ),
                    isSuccess: false,
                    kind: .evidenceWarning,
                    isEphemeral: true
                )
            } else {
                log(
                    "\(localized("失败", "Failed")): \(fileURL.lastPathComponent)",
                    detail: localized("预检失败", "Precheck failed") + ": \(awmError.localizedDescription)",
                    isSuccess: false,
                    kind: .resultError
                )
                state.failureCount += 1
                if queueIndex < selectedFiles.count {
                    selectedFiles.remove(at: queueIndex)
                }
                return true
            }
        } catch {
            log(
                "\(localized("失败", "Failed")): \(fileURL.lastPathComponent)",
                detail: localized("预检失败", "Precheck failed") + ": \(error.localizedDescription)",
                isSuccess: false,
                kind: .resultError
            )
            state.failureCount += 1
            if queueIndex < selectedFiles.count {
                selectedFiles.remove(at: queueIndex)
            }
            return true
        }
        return false
    }

    /// 嵌入阶段：进度轮询 + 嵌入 + SNR 日志。成功/失败均在内部记录日志并更新 state。
    func runEmbedPhase(
        audio: AWMAudio,
        audioBox: UnsafeAudioBox,
        fileURL: URL,
        resolvedTag: String,
        key: Data,
        activeKeySlot: UInt8,
        suffix: String,
        fileProgress: Double,
        updateFileProgress: @escaping (Double) -> Void,
        state: inout EmbedLoopState
    ) async {
        do {
            let baseName = fileURL.deletingPathExtension().lastPathComponent
            let ext = normalizedOutputExtension(from: fileURL.pathExtension)
            let outputDir = outputDirectory ?? fileURL.deletingLastPathComponent()
            let outputURL = outputDir.appendingPathComponent("\(baseName)\(suffix).\(ext)")
            audio.clearProgress()
            let pollTask = startProgressPolling(
                audio: audioBox,
                expectedOperation: .embed,
                profile: .embed,
                base: 0,
                span: 1,
                initialProgress: fileProgress,
                onProgress: updateFileProgress
            )
            let step: EmbedStepOutput
            do {
                step = try await Self.performEmbedStep(
                    audio: audioBox,
                    fileURL: fileURL,
                    outputURL: outputURL,
                    tagValue: resolvedTag,
                    key: key,
                    keySlot: activeKeySlot,
                    strength: UInt8(strength)
                )
            } catch {
                pollTask.cancel()
                _ = await pollTask.result
                throw error
            }
            pollTask.cancel()
            _ = await pollTask.result
            updateFileProgress(1)
            if let evidenceError = step.evidenceErrorDescription {
                log(
                    localized("证据记录失败", "Evidence record failed"),
                    detail: "\(outputURL.lastPathComponent): \(evidenceError)",
                    isSuccess: false,
                    kind: .evidenceWarning,
                    isEphemeral: true
                )
            }
            var successDetail = "→ \(outputURL.lastPathComponent)"
            if step.snrStatus == "ok", let snrDb = step.snrDb {
                successDetail += String(format: " · SNR %.2f dB", snrDb)
            } else if let snrStatus = step.snrStatus, snrStatus != "ok" {
                let reason = step.snrDetail ?? snrStatus
                log(
                    localized("SNR 不可用", "SNR unavailable"),
                    detail: reason,
                    isSuccess: false,
                    kind: .evidenceWarning,
                    isEphemeral: true
                )
            }
            log(
                "\(localized("成功", "Success")): \(fileURL.lastPathComponent)",
                detail: successDetail,
                kind: .resultOk
            )
            state.successCount += 1
        } catch {
            log(
                "\(localized("失败", "Failed")): \(fileURL.lastPathComponent)",
                detail: error.localizedDescription,
                isSuccess: false,
                kind: .resultError
            )
            state.failureCount += 1
        }
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
