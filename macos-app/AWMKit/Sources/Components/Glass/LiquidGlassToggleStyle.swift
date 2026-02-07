import SwiftUI

struct LiquidGlassToggleStyle: ToggleStyle {
    @Environment(\.colorScheme) private var colorScheme

    func makeBody(configuration: Configuration) -> some View {
        let cornerRadius = DesignSystem.CornerRadius.toggle
        let shape = RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)

        HStack(spacing: DesignSystem.Spacing.item) {
            Button(action: { withAnimation(DesignSystem.Animation.spring) { configuration.isOn.toggle() } }) {
                ZStack(alignment: configuration.isOn ? .trailing : .leading) {
                    shape
                        .fill(
                            configuration.isOn
                                ? (colorScheme == .light
                                    ? Color.accentColor.opacity(0.08)
                                    : Color.white.opacity(0.12))
                                : (colorScheme == .light
                                    ? Color.black.opacity(0.08)
                                    : Color.white.opacity(0.08))
                        )
                        .glassEffect(.regular, in: shape)
                        .frame(width: DesignSystem.Toggle.width, height: DesignSystem.Toggle.height)
                        .overlay(
                            shape.stroke(
                                DesignSystem.Colors.border(colorScheme),
                                lineWidth: DesignSystem.BorderWidth.standard
                            )
                        )
                    Circle()
                        .fill(Color.white.opacity(colorScheme == .dark ? 0.95 : 0.98))
                        .frame(width: DesignSystem.Toggle.thumbSize, height: DesignSystem.Toggle.thumbSize)
                        .padding(DesignSystem.Toggle.thumbPadding)
                        .shadow(color: Color.black.opacity(0.2), radius: 1.5, x: 0, y: 0.5)
                }
                .frame(width: DesignSystem.Toggle.width, height: DesignSystem.Toggle.height)
                .contentShape(Rectangle())
                .animation(DesignSystem.Animation.spring, value: configuration.isOn)
            }
            .buttonStyle(.plain)

            Circle()
                .fill(.thinMaterial)
                .glassEffect(.regular, in: Circle())
                .overlay(
                    Circle()
                        .fill(configuration.isOn ? Color.accentColor.opacity(0.8) : Color.secondary.opacity(0.35))
                )
                .frame(width: DesignSystem.Toggle.indicatorSize, height: DesignSystem.Toggle.indicatorSize)
                .shadow(color: configuration.isOn ? Color.accentColor.opacity(0.35) : Color.clear, radius: 4)
                .accessibilityHidden(true)
        }
    }
}
