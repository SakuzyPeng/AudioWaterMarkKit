import SwiftUI

/// AWMKit 设计系统 - 玻璃效果风格
struct DesignSystem {

    // MARK: - 窗口尺寸
    struct Window {
        static let minWidth: CGFloat = 1280
        static let minHeight: CGFloat = 800
        static let defaultWidth: CGFloat = 1280
        static let defaultHeight: CGFloat = 880
    }

    // MARK: - 间距
    struct Spacing {
        static let grid: CGFloat = 24
        static let card: CGFloat = grid
        static let horizontal: CGFloat = grid
        static let vertical: CGFloat = grid
        static let section: CGFloat = grid * 2.0 / 3.0
        static let item: CGFloat = grid / 2.0
        static let compact: CGFloat = grid / 3.0
    }

    // MARK: - 内边距
    struct Padding {
        static let card: CGFloat = 12
        static let row: CGFloat = 14
        static let button: (horizontal: CGFloat, vertical: CGFloat) = (16, 8)
        static let capsule: (horizontal: CGFloat, vertical: CGFloat) = (10, 4)
        static let toggle: CGFloat = 2
    }

    // MARK: - 边框宽度
    struct BorderWidth {
        static let standard: CGFloat = 1
        static let thin: CGFloat = 0.5
    }

    // MARK: - 颜色
    struct Colors {
        /// 卡片背景色
        static func cardBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.08)
                : Color.white.opacity(1.0)
        }

        /// 行条目背景色
        static func rowBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.08)
                : Color.black.opacity(0.02)
        }

        /// 按钮背景色
        static func buttonBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.12)
                : Color.white.opacity(0.95)
        }

        /// 边框颜色
        static func border(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.25)
                : Color.black.opacity(0.15)
        }

        /// Toggle 背景色
        static func toggleBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.15)
                : Color.white.opacity(0.9)
        }

        /// 胶囊背景色
        static func capsuleBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.08)
                : Color.black.opacity(0.05)
        }

        /// 胶囊边框色
        static func capsuleBorder(_ colorScheme: ColorScheme) -> Color {
            Color.primary.opacity(colorScheme == .dark ? 0.15 : 0.1)
        }

        /// 卡片标题栏背景色
        static func titleBarBackground(_ colorScheme: ColorScheme) -> Color {
            colorScheme == .dark
                ? Color.white.opacity(0.06)
                : Color(red: 247/255, green: 249/255, blue: 252/255)
        }

        /// 成功色
        static let success = Color.green
        /// 警告色
        static let warning = Color.orange
        /// 失败色
        static let error = Color.red
    }

    // MARK: - 阴影
    struct Shadows {
        /// 卡片阴影
        static func card(_ colorScheme: ColorScheme) -> ShadowStyle {
            ShadowStyle(
                color: Color.black.opacity(colorScheme == .dark ? 0.4 : 0.1),
                radius: 20,
                x: 0,
                y: 10
            )
        }

        /// 行条目阴影
        static func row(_ colorScheme: ColorScheme) -> ShadowStyle {
            ShadowStyle(
                color: Color.black.opacity(colorScheme == .dark ? 0.35 : 0.12),
                radius: 6,
                x: 0,
                y: 3
            )
        }

        /// 按钮阴影
        static func button(_ colorScheme: ColorScheme) -> ShadowStyle {
            ShadowStyle(
                color: Color.black.opacity(colorScheme == .dark ? 0.4 : 0.12),
                radius: 16,
                x: 0,
                y: 8
            )
        }

        /// 胶囊阴影
        static func capsule(_ colorScheme: ColorScheme) -> ShadowStyle {
            ShadowStyle(
                color: Color.black.opacity(colorScheme == .dark ? 0.3 : 0.08),
                radius: 8,
                x: 0,
                y: 4
            )
        }
    }

    // MARK: - 圆角
    struct CornerRadius {
        static let card: CGFloat = 22
        static let button: CGFloat = 18
        static let row: CGFloat = 18
        static let toggle: CGFloat = 16
    }

    // MARK: - Toggle 样式
    struct Toggle {
        static let width: CGFloat = 50
        static let height: CGFloat = 26
        static let thumbSize: CGFloat = 22
        static let thumbPadding: CGFloat = 2
        static let indicatorSize: CGFloat = 12
    }

    // MARK: - 动画参数
    struct Animation {
        /// 标准弹簧动画
        static let spring = SwiftUI.Animation.spring(response: 0.25, dampingFraction: 0.8)

        /// 按钮按下动画
        static let buttonPress = SwiftUI.Animation.spring(response: 0.2, dampingFraction: 0.85)

        /// 按钮缩放比例
        static let buttonPressScale: CGFloat = 0.97
    }

    // MARK: - 字体样式
    struct Typography {
        static let buttonLabel = Font.subheadline.weight(.semibold)
        static let capsuleLabel = Font.caption2.weight(.semibold)
    }
}

/// 阴影样式
struct ShadowStyle {
    let color: Color
    let radius: CGFloat
    let x: CGFloat
    let y: CGFloat
}

// MARK: - View 扩展
extension View {
    /// 应用阴影样式
    func shadow(_ style: ShadowStyle) -> some View {
        self.shadow(color: style.color, radius: style.radius, x: style.x, y: style.y)
    }
}
