import SwiftUI

/// 三层 HDR 发光文字效果（仅暗色模式生效）
///
/// 参考 AWMKitGlass 的发光实现：
/// - 外层：环境光晕（blur 大、低不透明度）
/// - 中层：高能抗锯齿（displayP3 超白、blur 小）
/// - 内层：实色内核
struct HDRGlowText: View {
    let text: String
    let font: Font
    let intensity: Intensity

    enum Intensity {
        /// 高强度：用于正在处理的文件名（displayP3 2.0, blur 1.5/0.3）
        case high
        /// 中等强度：用于日志标题关键词（displayP3 1.8, blur 1.2/0.2）
        case medium
    }

    var body: some View {
        switch intensity {
        case .high:
            highIntensityGlow
        case .medium:
            mediumIntensityGlow
        }
    }

    private var highIntensityGlow: some View {
        ZStack {
            Text(text).font(font)
                .foregroundStyle(.white)
                .blur(radius: 1.5)
                .opacity(0.2)
                .blendMode(.plusLighter)

            Text(text).font(font)
                .foregroundStyle(Color(.displayP3, red: 2.0, green: 2.0, blue: 2.0))
                .blur(radius: 0.3)
                .opacity(0.6)
                .blendMode(.plusLighter)

            Text(text).font(font)
                .foregroundStyle(.white)
        }
        .allowedDynamicRange(.high)
        .compositingGroup()
    }

    private var mediumIntensityGlow: some View {
        ZStack {
            Text(text).font(font)
                .foregroundStyle(.primary)
                .blur(radius: 1.2)
                .opacity(0.15)
                .blendMode(.plusLighter)

            Text(text).font(font)
                .foregroundStyle(Color(.displayP3, red: 1.8, green: 1.8, blue: 1.8))
                .blur(radius: 0.2)
                .opacity(0.5)
                .blendMode(.plusLighter)

            Text(text).font(font)
                .foregroundStyle(.primary)
        }
        .allowedDynamicRange(.high)
        .compositingGroup()
    }
}
