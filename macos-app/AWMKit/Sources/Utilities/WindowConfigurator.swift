import AppKit
import SwiftUI

struct WindowConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        DispatchQueue.main.async {
            context.coordinator.configureIfNeeded(window: view.window)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async {
            context.coordinator.configureIfNeeded(window: nsView.window)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator {
        private var didConfigure = false

        func configureIfNeeded(window: NSWindow?) {
            guard !didConfigure, let window else { return }
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
        }
    }
}
