import SwiftUI

/// 液态玻璃风格的进度条
struct LiquidGlassProgressView: View {
    @Environment(\.colorScheme) private var colorScheme
    let progress: Double
    let height: CGFloat
    @State private var isFlashing = false

    init(progress: Double, height: CGFloat = 6) {
        self.progress = max(0, min(1, progress))
        self.height = height
    }

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                // 背景轨道
                Capsule()
                    .fill(
                        colorScheme == .dark
                            ? Color.white.opacity(0.08)
                            : Color.black.opacity(0.05)
                    )
                    .glassEffect(.regular, in: .capsule)
                    .overlay(
                        Capsule()
                            .stroke(
                                colorScheme == .light
                                    ? Color.black.opacity(0.12)
                                    : DesignSystem.Colors.border(colorScheme),
                                lineWidth: colorScheme == .light ? 1 : DesignSystem.BorderWidth.thin
                            )
                    )

                // 进度条
                Capsule()
                    .fill(
                        LinearGradient(
                            colors: [
                                Color.accentColor.opacity(0.9),
                                Color.accentColor
                            ],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .glassEffect(.regular, in: .capsule)
                    .shadow(
                        color: progress == 1.0 && colorScheme == .dark
                            ? Color(.displayP3, red: isFlashing ? 1.5 : 1.0, green: isFlashing ? 1.5 : 1.0, blue: isFlashing ? 1.5 : 1.0)
                                .opacity(isFlashing ? 0.3 : 0.4)
                            : Color.accentColor.opacity(0.4),
                        radius: isFlashing && colorScheme == .dark ? 5 : 4,
                        x: 0,
                        y: 0
                    )
                    .allowedDynamicRange(progress == 1.0 && colorScheme == .dark ? .high : .standard)
                    .animation(.easeOut(duration: 1.2), value: isFlashing)
                    .frame(width: geometry.size.width * progress)
                    .animation(.easeInOut(duration: 0.4), value: progress)
            }
        }
        .frame(height: height)
        .onChange(of: progress) { oldValue, newValue in
            if newValue == 1.0 && oldValue < 1.0 {
                isFlashing = true
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
                    isFlashing = false
                }
            }
        }
    }
}
