import SwiftUI

/// 文件队列条目行 - 带 HDR 发光效果（暗色模式下处理中文件发光）
struct FileEntryRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let name: String
    let detail: String
    let statusText: String
    let isProcessing: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top) {
                if isProcessing {
                    if colorScheme == .dark {
                        HDRGlowText(
                            text: name,
                            font: .subheadline.weight(.semibold),
                            intensity: .high
                        )
                    } else {
                        Text(name)
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.primary)
                    }
                } else {
                    Text(name)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.primary)
                }

                Spacer()
                StatusCapsule(status: statusText, isHighlight: isProcessing)
            }

            if !detail.isEmpty {
                Text(detail)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .lineLimit(2)
                    .truncationMode(.middle)
            }
        }
        .entryRowStyle()
        .overlay {
            if isProcessing && colorScheme == .light {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row)
                    .stroke(Color.black.opacity(0.15), lineWidth: 1)
            }
        }
    }
}
