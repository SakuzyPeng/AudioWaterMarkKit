import AppKit
import SwiftUI

struct WindowConfigurator: NSViewRepresentable {
    @ObservedObject var appState: AppState

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            context.coordinator.configureIfNeeded(window: view.window, appState: appState)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            context.coordinator.configureIfNeeded(window: nsView.window, appState: appState)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator {
        private var didConfigure = false
        private weak var accessoryHostView: NSHostingView<TitlebarStatusAccessoryView>?

        func configureIfNeeded(window: NSWindow?, appState: AppState) {
            guard let window else { return }

            if !didConfigure {
                didConfigure = true

                let minWidth = DesignSystem.Window.minWidth
                let minHeight = DesignSystem.Window.minHeight
                window.minSize = NSSize(width: minWidth, height: minHeight)

                var frame = window.frame
                if frame.size.width < minWidth || frame.size.height < minHeight {
                    frame.size.width = max(frame.size.width, minWidth)
                    frame.size.height = max(frame.size.height, minHeight)

                    if let visible = window.screen?.visibleFrame ?? NSScreen.main?.visibleFrame {
                        if frame.maxX > visible.maxX { frame.origin.x = visible.maxX - frame.size.width }
                        if frame.minX < visible.minX { frame.origin.x = visible.minX }
                        if frame.maxY > visible.maxY { frame.origin.y = visible.maxY - frame.size.height }
                        if frame.minY < visible.minY { frame.origin.y = visible.minY }
                    }

                    window.setFrame(frame, display: true)
                }

                installTitlebarAccessory(window: window, appState: appState)
            }

            accessoryHostView?.rootView = TitlebarStatusAccessoryView(appState: appState)
        }

        private func installTitlebarAccessory(window: NSWindow, appState: AppState) {
            let hostingView = NSHostingView(rootView: TitlebarStatusAccessoryView(appState: appState))
            hostingView.frame = NSRect(x: 0, y: 0, width: 108, height: 24)

            let accessory = NSTitlebarAccessoryViewController()
            accessory.layoutAttribute = .right
            accessory.view = hostingView
            window.addTitlebarAccessoryViewController(accessory)
            self.accessoryHostView = hostingView
        }
    }
}

private struct TitlebarStatusAccessoryView: View {
    @ObservedObject var appState: AppState
    @State private var isHoveringKey = false
    @State private var isHoveringAudio = false
    @State private var isHoveringDatabase = false
    @State private var keyTapFlash = false
    @State private var audioTapFlash = false
    @State private var databaseTapFlash = false

    var body: some View {
        HStack(spacing: 8) {
            statusIconButton(
                systemName: "key.fill",
                tone: appState.keyStatusTone,
                isHovering: isHoveringKey,
                isFlashing: keyTapFlash,
                accessibilityLabel: "密钥状态"
            ) {
                flashTapFeedback(for: .key)
                Task { await appState.handleKeyIndicatorTap() }
            }
            .popover(isPresented: $isHoveringKey, arrowEdge: .bottom) {
                Text(appState.keyStatusHelp)
                    .font(.caption)
                    .foregroundStyle(.primary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 8)
                    .frame(maxWidth: 280, alignment: .leading)
            }
            .onHover { hovering in
                isHoveringKey = hovering
                if hovering {
                    isHoveringAudio = false
                }
            }

            statusIconButton(
                systemName: "waveform",
                tone: appState.audioStatusTone,
                isHovering: isHoveringAudio,
                isFlashing: audioTapFlash,
                accessibilityLabel: "AudioWmark 状态"
            ) {
                flashTapFeedback(for: .audio)
                appState.checkAudioStatus()
            }
            .popover(isPresented: $isHoveringAudio, arrowEdge: .bottom) {
                Text(appState.audioStatusHelp)
                    .font(.caption)
                    .foregroundStyle(.primary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 8)
                    .frame(maxWidth: 280, alignment: .leading)
            }
            .onHover { hovering in
                isHoveringAudio = hovering
                if hovering {
                    isHoveringKey = false
                }
            }

            statusIconButton(
                systemName: "externaldrive.fill",
                tone: appState.databaseStatusTone,
                isHovering: isHoveringDatabase,
                isFlashing: databaseTapFlash,
                accessibilityLabel: "数据库状态"
            ) {
                flashTapFeedback(for: .database)
                appState.checkDatabaseStatus()
            }
            .popover(isPresented: $isHoveringDatabase, arrowEdge: .bottom) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("映射总数：\(appState.mappingCount)")
                    Text("证据总数（SHA256+指纹）：\(appState.evidenceCount)")
                }
                .font(.caption)
                .foregroundStyle(.primary)
                .padding(.horizontal, 10)
                .padding(.vertical, 8)
                .frame(maxWidth: 320, alignment: .leading)
            }
            .onHover { hovering in
                isHoveringDatabase = hovering
                if hovering {
                    isHoveringKey = false
                    isHoveringAudio = false
                }
            }
        }
        .frame(height: 24)
    }

    private enum TapTarget {
        case key
        case audio
        case database
    }

    @ViewBuilder
    private func statusIconButton(
        systemName: String,
        tone: AppState.RuntimeStatusTone,
        isHovering: Bool,
        isFlashing: Bool,
        accessibilityLabel: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(statusColor(tone))
                .frame(width: 24, height: 22)
                .background(backgroundColor(isHovering: isHovering, isFlashing: isFlashing))
                .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
                .overlay {
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .strokeBorder(borderColor(isHovering: isHovering), lineWidth: isHovering ? 1 : 0)
                }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(accessibilityLabel)
    }

    private func backgroundColor(isHovering: Bool, isFlashing: Bool) -> Color {
        if isFlashing {
            return .accentColor.opacity(0.25)
        }
        if isHovering {
            return .primary.opacity(0.12)
        }
        return .clear
    }

    private func borderColor(isHovering: Bool) -> Color {
        isHovering ? .primary.opacity(0.24) : .clear
    }

    private func flashTapFeedback(for target: TapTarget) {
        switch target {
        case .key:
            keyTapFlash = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) {
                keyTapFlash = false
            }
        case .audio:
            audioTapFlash = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) {
                audioTapFlash = false
            }
        case .database:
            databaseTapFlash = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) {
                databaseTapFlash = false
            }
        }
    }

    private func statusColor(_ tone: AppState.RuntimeStatusTone) -> Color {
        switch tone {
        case .ready:
            return .green
        case .warning:
            return .orange
        case .error:
            return .red
        case .unknown:
            return .secondary
        }
    }
}
