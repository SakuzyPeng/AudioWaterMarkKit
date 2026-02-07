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
    @Published private(set) var keyStatusTone: RuntimeStatusTone = .unknown
    @Published private(set) var keyStatusHelp: String = "密钥状态检查中..."
    @Published private(set) var audioStatusTone: RuntimeStatusTone = .unknown
    @Published private(set) var audioStatusHelp: String = "AudioWmark 状态检查中..."

    let audio: AWMAudio?
    let keychain = AWMKeychain()
    private let audioInitError: String?

    enum Tab: String, CaseIterable, Identifiable {
        case embed = "嵌入"
        case detect = "检测"
        case tags = "标签"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .embed: return "waveform.badge.plus"
            case .detect: return "waveform.badge.magnifyingglass"
            case .tags: return "tag"
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
        Task {
            await refreshRuntimeStatus()
        }
    }

    func refreshRuntimeStatus() async {
        await checkKey()
        checkAudioStatus()
    }

    func checkKey() async {
        do {
            if let key = try keychain.loadKey() {
                keyLoaded = true
                keyStatusTone = .ready
                keyStatusHelp = "密钥已配置（\(key.count) 字节）"
            } else {
                keyLoaded = false
                keyStatusTone = .warning
                keyStatusHelp = "密钥未配置，点击自动生成"
            }
        } catch {
            keyLoaded = false
            keyStatusTone = .error
            keyStatusHelp = "密钥读取失败：\(error.localizedDescription)"
        }
    }

    func handleKeyIndicatorTap() async {
        do {
            if try keychain.loadKey() == nil {
                _ = try keychain.generateAndSaveKey()
            }
            await checkKey()
        } catch {
            keyLoaded = false
            keyStatusTone = .error
            keyStatusHelp = "密钥初始化失败：\(error.localizedDescription)"
        }
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

    private func inferredAudioBackend() -> String {
        let bundledBinary = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent("bundled", isDirectory: true)
            .appendingPathComponent("bin", isDirectory: true)
            .appendingPathComponent("audiowmark", isDirectory: false)
            .path
        return FileManager.default.isExecutableFile(atPath: bundledBinary) ? "bundled" : "PATH"
    }
}
