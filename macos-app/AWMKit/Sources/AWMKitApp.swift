import SwiftUI
import AppKit
import AWMKit

@main
struct AWMKitApp: App {
    @StateObject private var appState = AppState()
    @AppStorage("appearanceMode") private var appearanceMode: AppearanceMode = .system

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .environment(\.locale, appState.uiLocale)
                .frame(
                    minWidth: DesignSystem.Window.minWidth,
                    idealWidth: DesignSystem.Window.defaultWidth,
                    minHeight: DesignSystem.Window.minHeight,
                    idealHeight: DesignSystem.Window.defaultHeight
                )
                .onAppear { applyAppearance() }
                .onChange(of: appearanceMode) { _, _ in
                    applyAppearance()
                }
        }
        .defaultSize(width: DesignSystem.Window.defaultWidth, height: DesignSystem.Window.defaultHeight)
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }

    private func applyAppearance() {
        switch appearanceMode {
        case .system:
            NSApp.appearance = nil
        case .light:
            NSApp.appearance = NSAppearance(named: .aqua)
        case .dark:
            NSApp.appearance = NSAppearance(named: .darkAqua)
        }
    }
}

/// 全局应用状态
@MainActor
class AppState: ObservableObject {
    enum RuntimeStatusTone {
        case ready
        case warning
        case error
        case unknown
    }

    @Published var selectedTab: Tab = .embed
    @Published var isProcessing = false
    @Published var keyLoaded = false
    @Published private(set) var keySourceLabel: String = "未配置"
    @Published var activeKeySlot: Int = 0
    @Published private(set) var keyStatusTone: RuntimeStatusTone = .unknown
    @Published private(set) var keyStatusHelp: String = "密钥状态检查中..."
    @Published private(set) var audioStatusTone: RuntimeStatusTone = .unknown
    @Published private(set) var audioStatusHelp: String = "AudioWmark 状态检查中..."
    @Published private(set) var databaseStatusTone: RuntimeStatusTone = .unknown
    @Published private(set) var databaseStatusHelp: String = "数据库状态检查中..."
    @Published private(set) var mappingCount: Int = 0
    @Published private(set) var evidenceCount: Int = 0
    @Published var uiLanguage: UILanguageOption = .zhCN

    let audio: AWMAudio?
    private let audioInitError: String?

    enum Tab: String, CaseIterable, Identifiable {
        case embed
        case detect
        case tags
        case key

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .embed: return "waveform.badge.plus"
            case .detect: return "waveform.badge.magnifyingglass"
            case .tags: return "tag"
            case .key: return "key"
            }
        }
    }

    init() {
        uiLanguage = Self.resolveInitialLanguage()

        do {
            let instance = try AWMAudio()
            self.audio = instance
            self.audioInitError = nil
        } catch {
            self.audio = nil
            self.audioInitError = error.localizedDescription
        }

        checkAudioStatus()
        checkDatabaseStatus()
        loadActiveKeySlot()
        Task {
            await refreshRuntimeStatus()
        }
    }

    var uiLocale: Locale {
        uiLanguage.locale
    }

    func localizedTabTitle(_ tab: Tab) -> String {
        switch tab {
        case .embed:
            return l("嵌入", "Embed")
        case .detect:
            return l("检测", "Detect")
        case .tags:
            return l("标签", "Database")
        case .key:
            return l("密钥", "Keys")
        }
    }

    func setUILanguage(_ language: UILanguageOption) {
        guard uiLanguage != language else { return }
        do {
            try AWMUILanguageStore.set(AWMUILanguage(rawValue: language.rawValue))
            uiLanguage = language
            Task { [weak self] in
                await self?.refreshRuntimeStatus()
            }
        } catch {
            // Keep previous UI language when persistence fails.
        }
    }

    func refreshRuntimeStatus() async {
        loadActiveKeySlot()
        await checkKey()
        checkAudioStatus()
        checkDatabaseStatus()
    }

    func checkKey() async {
        let resolvedActiveSlot: Int
        if let slot = try? AWMKeyStore.activeSlot() {
            resolvedActiveSlot = Int(slot)
            activeKeySlot = resolvedActiveSlot
        } else {
            resolvedActiveSlot = activeKeySlot
        }
        let slotSummaries = (try? AWMKeyStore.slotSummaries()) ?? []

        do {
            if !AWMKeyStore.exists() {
                keyLoaded = false
                keySourceLabel = l("未配置", "Not configured")
                keyStatusTone = .warning
                keyStatusHelp = formatKeyStatusHelp(
                    activeSlot: resolvedActiveSlot,
                    summaries: slotSummaries,
                    keyAvailable: false
                )
                return
            }

            _ = try AWMKeyStore.loadActiveKey()
            keyLoaded = true

            let backend = try? AWMKeyStore.backendLabel()
            if let backend, !backend.isEmpty, backend != "none" {
                keySourceLabel = backend
            } else {
                keySourceLabel = l("已配置（来源未知）", "Configured (unknown backend)")
            }

            keyStatusTone = .ready
            keyStatusHelp = formatKeyStatusHelp(
                activeSlot: resolvedActiveSlot,
                summaries: slotSummaries,
                keyAvailable: true
            )
        } catch {
            keyLoaded = false
            keySourceLabel = l("读取失败", "Read failed")
            keyStatusTone = .error
            keyStatusHelp = "\(l("密钥读取失败", "Key read failed")): \(error.localizedDescription)"
        }
    }

    func handleKeyIndicatorTap() async {
        await refreshRuntimeStatus()
    }

    func checkAudioStatus() {
        guard let audio else {
            audioStatusTone = .error
            audioStatusHelp = "\(l("AudioWmark 初始化失败", "AudioWmark initialization failed")): \(audioInitError ?? l("未找到可用二进制", "No available binary"))"
            return
        }

        guard audio.isAvailable else {
            audioStatusTone = .error
            audioStatusHelp = l("AudioWmark 不可用：初始化成功但无法执行", "AudioWmark unavailable: initialized but execution failed")
            return
        }

        audioStatusTone = .ready
        audioStatusHelp = "\(l("AudioWmark 可用", "AudioWmark available")) (\(inferredAudioBackend()))"
    }

    func checkDatabaseStatus() {
        do {
            let summary = try AWMDatabaseStore.summary()
            mappingCount = summary.tagCount
            evidenceCount = summary.evidenceCount
            databaseStatusTone = (summary.tagCount == 0 && summary.evidenceCount == 0) ? .warning : .ready
            databaseStatusHelp = """
            \(l("映射总数", "Total mappings")): \(summary.tagCount)
            \(l("证据总数（SHA256+指纹）", "Total evidence (SHA256 + fingerprint)")): \(summary.evidenceCount)
            """
        } catch {
            mappingCount = 0
            evidenceCount = 0
            databaseStatusTone = .error
            databaseStatusHelp = "\(l("数据库读取失败", "Database read failed")): \(error.localizedDescription)"
        }
    }

    private func inferredAudioBackend() -> String {
        let bundledBinary = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent("bundled", isDirectory: true)
            .appendingPathComponent("bin", isDirectory: true)
            .appendingPathComponent("audiowmark", isDirectory: false)
            .path
        return FileManager.default.isExecutableFile(atPath: bundledBinary) ? "bundled" : "PATH"
    }

    func generateKey(slot: Int) async throws {
        let normalized = max(0, min(31, slot))
        _ = try AWMKeyStore.generateAndSaveKey(slot: UInt8(normalized))
        await refreshRuntimeStatus()
    }

    func deleteKey(slot: Int) async throws {
        let normalized = max(0, min(31, slot))
        _ = try AWMKeyStore.deleteKey(slot: UInt8(normalized))
        await refreshRuntimeStatus()
    }

    func setSlotLabel(slot: Int, label: String) async throws {
        let normalized = max(0, min(31, slot))
        try AWMKeyStore.setSlotLabel(slot: UInt8(normalized), label: label)
        await refreshRuntimeStatus()
    }

    func clearSlotLabel(slot: Int) async throws {
        let normalized = max(0, min(31, slot))
        try AWMKeyStore.clearSlotLabel(slot: UInt8(normalized))
        await refreshRuntimeStatus()
    }

    func loadActiveKey() throws -> Data {
        try AWMKeyStore.loadActiveKey()
    }

    func loadActiveKeySlot() {
        do {
            activeKeySlot = Int(try AWMKeyStore.activeSlot())
        } catch {
            activeKeySlot = 0
        }
    }

    func setActiveKeySlot(_ slot: Int) {
        do {
            let normalized = max(0, min(31, slot))
            try AWMKeyStore.setActiveSlot(UInt8(normalized))
            activeKeySlot = Int(try AWMKeyStore.activeSlot())
            Task { [weak self] in
                await self?.refreshRuntimeStatus()
            }
        } catch {
            // Ignore setting persistence failure in UI state update path.
        }
    }

    private static func resolveInitialLanguage() -> UILanguageOption {
        if let stored = try? AWMUILanguageStore.get(),
           let resolved = UILanguageOption(rawValue: stored.rawValue) {
            return resolved
        }

        return UILanguageOption.defaultFromSystem()
    }

    private func l(_ zh: String, _ en: String) -> String {
        uiLanguage == .enUS ? en : zh
    }

    private func formatKeyStatusHelp(
        activeSlot: Int,
        summaries: [AWMKeySlotSummary],
        keyAvailable: Bool
    ) -> String {
        let configured = summaries.filter { $0.hasKey }
        let activeKeyId = summaries.first(where: { $0.slot == UInt8(activeSlot) })?.keyId ?? l("未配置", "Not configured")
        let listPreview = configured
            .prefix(6)
            .map { "\($0.slot):\($0.keyId ?? "-")" }
            .joined(separator: ", ")
        let slotDigest = configured.isEmpty
            ? "-"
            : (configured.count > 6 ? "\(listPreview), ..." : listPreview)
        let duplicateSlots = configured
            .filter { $0.statusText == "duplicate" }
            .map { String($0.slot) }
            .joined(separator: ",")

        var lines: [String] = [
            "\(l("激活槽位", "Active slot")): \(activeSlot)",
            "\(l("激活 Key ID", "Active Key ID")): \(activeKeyId)",
            "\(l("已配置槽位", "Configured slots")): \(configured.count)/32",
            "\(l("槽位摘要", "Slot summary")): \(slotDigest)"
        ]
        if !duplicateSlots.isEmpty {
            lines.append("\(l("重复密钥槽位", "Duplicate key slots")): \(duplicateSlots)")
        }
        lines.append(
            keyAvailable
                ? l("点击可刷新密钥状态", "Click to refresh key status")
                : l("未配置密钥，请前往“密钥”页面生成", "No key configured. Open Key page to generate one.")
        )
        return lines.joined(separator: "\n")
    }
}
