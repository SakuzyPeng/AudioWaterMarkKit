import SwiftUI

struct StatusCapsule: View {
    @Environment(\.colorScheme) private var colorScheme
    let status: String
    let isHighlight: Bool

    @ViewBuilder
    var textContent: some View {
        if isHighlight && colorScheme == .dark {
            ZStack {
                // 环境光晕
                Text(status)
                    .font(DesignSystem.Typography.capsuleLabel)
                    .padding(.horizontal, DesignSystem.Padding.capsule.horizontal)
                    .padding(.vertical, DesignSystem.Padding.capsule.vertical)
                    .foregroundStyle(.white)
                    .blur(radius: 0.6)
                    .opacity(0.15)
                    .blendMode(.plusLighter)

                // 高光层
                Text(status)
                    .font(DesignSystem.Typography.capsuleLabel)
                    .padding(.horizontal, DesignSystem.Padding.capsule.horizontal)
                    .padding(.vertical, DesignSystem.Padding.capsule.vertical)
                    .foregroundStyle(Color(.displayP3, red: 1.5, green: 1.5, blue: 1.5))
                    .blur(radius: 0.05)
                    .opacity(0.4)
                    .blendMode(.plusLighter)

                // 白色内核
                Text(status)
                    .font(DesignSystem.Typography.capsuleLabel)
                    .padding(.horizontal, DesignSystem.Padding.capsule.horizontal)
                    .padding(.vertical, DesignSystem.Padding.capsule.vertical)
                    .foregroundStyle(.white)
            }
            .allowedDynamicRange(.high)
            .compositingGroup()
        } else if isHighlight {
            Text(status)
                .font(DesignSystem.Typography.capsuleLabel)
                .padding(.horizontal, DesignSystem.Padding.capsule.horizontal)
                .padding(.vertical, DesignSystem.Padding.capsule.vertical)
                .foregroundStyle(.white)
        } else {
            Text(status)
                .font(DesignSystem.Typography.capsuleLabel)
                .padding(.horizontal, DesignSystem.Padding.capsule.horizontal)
                .padding(.vertical, DesignSystem.Padding.capsule.vertical)
                .foregroundStyle(Color.primary)
        }
    }

    var body: some View {
        textContent
        .background(
            Capsule()
                .fill(
                    isHighlight
                        ? Color.accentColor
                        : DesignSystem.Colors.capsuleBackground(colorScheme)
                )
                .glassEffect(.regular, in: .capsule)
                .overlay(
                    Capsule()
                        .stroke(
                            DesignSystem.Colors.capsuleBorder(colorScheme),
                            lineWidth: DesignSystem.BorderWidth.thin
                        )
                )
        )
    }
}
