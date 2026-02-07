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
    @Published var selectedTab: Tab = .embed
    @Published var isProcessing = false
    @Published var keyLoaded = false

    let audio: AWMAudio?
    let keychain = AWMKeychain()

    enum Tab: String, CaseIterable, Identifiable {
        case embed = "嵌入"
        case detect = "检测"
        case status = "状态"
        case tags = "标签"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .embed: return "waveform.badge.plus"
            case .detect: return "waveform.badge.magnifyingglass"
            case .status: return "info.circle"
            case .tags: return "tag"
            }
        }
    }

    init() {
        self.audio = try? AWMAudio()
        Task {
            await checkKey()
        }
    }

    func checkKey() async {
        do {
            _ = try keychain.loadKey()
            keyLoaded = true
        } catch {
            keyLoaded = false
        }
    }
}
