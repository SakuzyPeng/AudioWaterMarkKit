import SwiftUI

struct EntryRowStyle: ViewModifier {
    @Environment(\.colorScheme) private var colorScheme

    func body(content: Content) -> some View {
        let cornerRadius = DesignSystem.CornerRadius.row
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)

        content
            .padding(DesignSystem.Padding.row)
            .background {
                shape
                    .fill(DesignSystem.Colors.rowBackground(colorScheme))
                    .glassEffect(.regular, in: .rect(cornerRadius: cornerRadius))
                    .overlay {
                        shape.stroke(
                            DesignSystem.Colors.border(colorScheme),
                            lineWidth: DesignSystem.BorderWidth.standard
                        )
                    }
                    .shadow(DesignSystem.Shadows.row(colorScheme))
            }
            .clipShape(shape)
    }
}

extension View {
    func entryRowStyle() -> some View {
        modifier(EntryRowStyle())
    }
}
