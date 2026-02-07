import SwiftUI

struct HoverBackground<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    @State private var isHovered = false
    let content: () -> Content

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(
                    colorScheme == .dark
                        ? Color.white.opacity(0.06)
                        : Color.black.opacity(0.08)
                )
                .opacity(isHovered ? 1 : 0)
                .animation(.easeInOut(duration: 0.15), value: isHovered)
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .stroke(Color.white.opacity(colorScheme == .dark ? 0.15 : 0.1), lineWidth: isHovered ? 1 : 0)
                )
            content()
        }
        .contentShape(Rectangle())
        .onHover { hovering in
            isHovered = hovering
        }
    }
}
