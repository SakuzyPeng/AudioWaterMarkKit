import SwiftUI
import AppKit
import AWMKit
import SQLite3

@main
struct AWMKitApp: App {
    @StateObject private var appState = AppState()
    @AppStorage("appearanceMode") private var appearanceMode: AppearanceMode = .system

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
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

    let audio: AWMAudio?
    let keychain = AWMKeychain()
    private let audioInitError: String?

    enum Tab: String, CaseIterable, Identifiable {
        case embed = "嵌入"
        case detect = "检测"
        case tags = "标签"
        case key = "密钥"

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

    func refreshRuntimeStatus() async {
        await checkKey()
        checkAudioStatus()
        checkDatabaseStatus()
        loadActiveKeySlot()
    }

    func checkKey() async {
        do {
            if let key = try keychain.loadKey() {
                keyLoaded = true
                keySourceLabel = "macOS Keychain"
                keyStatusTone = .ready
                keyStatusHelp = "密钥已配置（\(key.count) 字节）"
            } else {
                keyLoaded = false
                keySourceLabel = "未配置"
                keyStatusTone = .warning
                keyStatusHelp = "密钥未配置，请前往“密钥”页面生成"
            }
        } catch {
            keyLoaded = false
            keySourceLabel = "读取失败"
            keyStatusTone = .error
            keyStatusHelp = "密钥读取失败：\(error.localizedDescription)"
        }
    }

    func handleKeyIndicatorTap() async {
        await refreshRuntimeStatus()
    }

    func checkAudioStatus() {
        guard let audio else {
            audioStatusTone = .error
            audioStatusHelp = "AudioWmark 初始化失败：\(audioInitError ?? "未找到可用二进制")"
            return
        }

        guard audio.isAvailable else {
            audioStatusTone = .error
            audioStatusHelp = "AudioWmark 不可用：初始化成功但无法执行"
            return
        }

        audioStatusTone = .ready
        audioStatusHelp = "AudioWmark 可用（\(inferredAudioBackend())）"
    }

    func checkDatabaseStatus() {
        do {
            let summary = try queryDatabaseSummary()
            mappingCount = summary.mappingCount
            evidenceCount = summary.evidenceCount
            databaseStatusTone = (summary.mappingCount == 0 && summary.evidenceCount == 0) ? .warning : .ready
            databaseStatusHelp = """
            映射总数：\(summary.mappingCount)
            证据总数（SHA256+指纹）：\(summary.evidenceCount)
            """
        } catch {
            mappingCount = 0
            evidenceCount = 0
            databaseStatusTone = .error
            databaseStatusHelp = "数据库读取失败：\(error.localizedDescription)"
        }
    }

    private struct DatabaseSummary {
        let mappingCount: Int
        let evidenceCount: Int
    }

    private enum DatabaseStatusError: LocalizedError {
        case openFailed(String)
        case prepareFailed(String)
        case stepFailed(String)

        var errorDescription: String? {
            switch self {
            case .openFailed(let message):
                return "打开失败：\(message)"
            case .prepareFailed(let message):
                return "预处理失败：\(message)"
            case .stepFailed(let message):
                return "查询失败：\(message)"
            }
        }
    }

    private func queryDatabaseSummary() throws -> DatabaseSummary {
        let dbURL = databaseURL()
        var db: OpaquePointer?
        guard sqlite3_open_v2(
            dbURL.path,
            &db,
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
            nil
        ) == SQLITE_OK else {
            let message = db.flatMap(sqliteMessage(from:)) ?? "未知错误"
            if let db { sqlite3_close(db) }
            throw DatabaseStatusError.openFailed(message)
        }
        defer { sqlite3_close(db) }

        let mappingCount = try queryTableCountIfExists(db: db, tableName: "tag_mappings")
        let evidenceCount = try queryTableCountIfExists(db: db, tableName: "audio_evidence")
        return DatabaseSummary(mappingCount: mappingCount, evidenceCount: evidenceCount)
    }

    private func queryTableCountIfExists(db: OpaquePointer?, tableName: String) throws -> Int {
        guard try tableExists(db: db, tableName: tableName) else {
            return 0
        }

        let sql = "SELECT COUNT(*) FROM \(tableName)"
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
            throw DatabaseStatusError.prepareFailed(sqliteMessage(from: db))
        }
        defer { sqlite3_finalize(statement) }

        let step = sqlite3_step(statement)
        guard step == SQLITE_ROW else {
            throw DatabaseStatusError.stepFailed(sqliteMessage(from: db))
        }
        return Int(sqlite3_column_int64(statement, 0))
    }

    private func tableExists(db: OpaquePointer?, tableName: String) throws -> Bool {
        let escaped = tableName.replacingOccurrences(of: "'", with: "''")
        let sql = "SELECT 1 FROM sqlite_master WHERE type='table' AND name='\(escaped)' LIMIT 1"
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
            throw DatabaseStatusError.prepareFailed(sqliteMessage(from: db))
        }
        defer { sqlite3_finalize(statement) }

        let step = sqlite3_step(statement)
        if step == SQLITE_ROW {
            return true
        }
        if step == SQLITE_DONE {
            return false
        }
        throw DatabaseStatusError.stepFailed(sqliteMessage(from: db))
    }

    private func sqliteMessage(from db: OpaquePointer?) -> String {
        guard let db, let cString = sqlite3_errmsg(db) else { return "未知 SQLite 错误" }
        return String(cString: cString)
    }

    private func databaseURL() -> URL {
        URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent("awmkit.db", isDirectory: false)
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

    func generateKey() async throws {
        _ = try keychain.generateAndSaveKey()
        await checkKey()
    }

    func deleteKey() async throws {
        try keychain.deleteKey()
        await checkKey()
    }

    func loadActiveKeySlot() {
        do {
            activeKeySlot = try AppSettingsStore.getActiveKeySlot()
        } catch {
            activeKeySlot = 0
        }
    }

    func setActiveKeySlot(_ slot: Int) {
        do {
            try AppSettingsStore.setActiveKeySlot(slot)
            activeKeySlot = max(0, min(31, slot))
        } catch {
            // Ignore setting persistence failure in UI state update path.
        }
    }
}
