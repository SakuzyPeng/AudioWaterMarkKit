import SwiftUI

struct GlassModifier: ViewModifier {
    @Environment(\.colorScheme) private var colorScheme

    func body(content: Content) -> some View {
        let cornerRadius = DesignSystem.CornerRadius.card
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)

        content
            .background(
                shape
                    .fill(DesignSystem.Colors.cardBackground(colorScheme))
                    .glassEffect(.regular, in: shape)
                    .overlay(
                        shape.stroke(DesignSystem.Colors.border(colorScheme), lineWidth: 1)
                    )
                    .shadow(DesignSystem.Shadows.card(colorScheme))
                    .clipShape(shape)
            )
            .backgroundExtensionEffect()
    }
}

extension View {
    /// 应用玻璃效果
    func glassBackground() -> some View {
        modifier(GlassModifier())
    }
}
