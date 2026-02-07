import SwiftUI

struct GlassButtonStyle: ButtonStyle {
    enum Size {
        case regular
        case compact
    }

    @Environment(\.colorScheme) private var colorScheme
    @State private var isHovered = false

    var accentOn: Bool = false
    var size: Size = .regular

    func makeBody(configuration: Configuration) -> some View {
        let isActive = accentOn || configuration.isPressed
        let cornerRadius = DesignSystem.CornerRadius.button
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
        let metrics = metrics(for: size)

        let strokeColor: Color
        let strokeWidth: CGFloat
        if colorScheme == .light {
            strokeColor = Color.black.opacity(isActive ? 0.25 : 0.12)
            strokeWidth = 1.2
        } else {
            strokeColor = Color.white.opacity(isActive ? 0.4 : 0.2)
            strokeWidth = DesignSystem.BorderWidth.standard
        }

        return configuration.label
            .font(metrics.font)
            .padding(.horizontal, metrics.horizontalPadding)
            .padding(.vertical, metrics.verticalPadding)
            .frame(minHeight: metrics.minHeight)
            .contentShape(shape)
            .background(
                shape
                    .fill(Color.clear)
                    .overlay(
                        shape.stroke(
                            strokeColor,
                            lineWidth: strokeWidth
                        )
                    )
                    .overlay(
                        shape.stroke(isActive ? Color.accentColor.opacity(0.35) : Color.clear, lineWidth: strokeWidth)
                    )
            )
            .offset(y: isHovered ? -3 : 0)
            .shadow(color: Color.black.opacity(isHovered ? 0.15 : 0.05), radius: isHovered ? 8 : 4, y: isHovered ? 6 : 2)
            .scaleEffect(configuration.isPressed ? DesignSystem.Animation.buttonPressScale : 1)
            .animation(.easeInOut(duration: 0.2), value: isHovered)
            .animation(DesignSystem.Animation.buttonPress, value: configuration.isPressed)
            .onHover { hovering in
                isHovered = hovering
            }
    }

    private struct ButtonMetrics {
        let font: Font
        let horizontalPadding: CGFloat
        let verticalPadding: CGFloat
        let minHeight: CGFloat
    }

    private func metrics(for size: Size) -> ButtonMetrics {
        switch size {
        case .regular:
            return ButtonMetrics(
                font: DesignSystem.Typography.buttonLabel,
                horizontalPadding: DesignSystem.Padding.button.horizontal,
                verticalPadding: DesignSystem.Padding.button.vertical,
                minHeight: 32
            )
        case .compact:
            return ButtonMetrics(
                font: Font.footnote.weight(.semibold),
                horizontalPadding: 12,
                verticalPadding: 6,
                minHeight: 28
            )
        }
    }
}
