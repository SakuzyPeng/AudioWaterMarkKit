import SwiftUI
import UniformTypeIdentifiers

struct DetectView: View {
    @EnvironmentObject var appState: AppState
    @ObservedObject var viewModel: DetectViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var isDropTargeted = false

    var body: some View {
        GeometryReader { proxy in
            let spacing = DesignSystem.Spacing.card
            let padH = DesignSystem.Spacing.horizontal
            let padV = DesignSystem.Spacing.vertical
            let rowHeight = (proxy.size.height - padV * 2 - spacing) / 2

            VStack(spacing: spacing) {
                HStack(alignment: .top, spacing: spacing) {
                    inputCard
                    GlassCard {
                        operationCard
                    }
                }
                .frame(maxWidth: .infinity)
                .frame(height: rowHeight)

                HStack(alignment: .top, spacing: spacing) {
                    fileListCard
                    logCard
                }
                .frame(maxWidth: .infinity)
                .frame(height: rowHeight)
            }
            .padding(.horizontal, padH)
            .padding(.vertical, padV)
            .frame(width: proxy.size.width, height: proxy.size.height, alignment: .top)
            .id(colorScheme)
        }
    }

    // MARK: - 输入卡片（左上）

    private var inputCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 14) {
                directorySummary

                HStack(spacing: 12) {
                    Button(action: { viewModel.selectFiles() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "folder")
                            Text("选择")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel("选择输入源")
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.detectFiles(audio: appState.audio) }) {
                        HStack(spacing: 6) {
                            Image(systemName: viewModel.isProcessing ? "stop.fill" : "play.fill")
                                .foregroundColor(viewModel.isProcessing ? .red : .accentColor)
                            Text(viewModel.isProcessing ? "停止" : "检测")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(accentOn: !viewModel.isProcessing, size: .compact))
                    .accessibilityLabel(viewModel.isProcessing ? "停止检测" : "开始检测")
                    .disabled(viewModel.selectedFiles.isEmpty || viewModel.isProcessing)

                    Button(action: { viewModel.clearQueue() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "xmark.circle")
                                .foregroundColor(viewModel.isClearQueueSuccess ? .green : .primary)
                            Text("清空")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel("清空队列")
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.clearLogs() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "line.3.horizontal.decrease.circle")
                                .foregroundColor(viewModel.isClearLogsSuccess ? .green : .primary)
                            Text("清空")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel("清空日志")
                }
                .frame(maxWidth: .infinity, alignment: .center)

                Divider().opacity(0.5)

                ZStack {
                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                        .strokeBorder(style: StrokeStyle(lineWidth: 1.5, dash: [6]))
                        .foregroundColor(isDropTargeted ? .accentColor : (colorScheme == .dark ? Color.white.opacity(0.25) : Color.black.opacity(0.2)))
                        .animation(.easeInOut(duration: 0.2), value: isDropTargeted)
                    VStack(spacing: 8) {
                        Image(systemName: "waveform.badge.magnifyingglass")
                            .font(.title2)
                        Text("拖拽音频文件到此处")
                            .font(.headline)
                        Text("检测文件中是否包含水印标签")
                            .font(.footnote)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 12)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .onDrop(of: [UTType.fileURL.identifier], isTargeted: $isDropTargeted) { providers in
            viewModel.processDropProviders(providers)
            return true
        }
        .id(colorScheme)
    }

    private var directorySummary: some View {
        VStack(spacing: 10) {
            directoryInfoRow(
                title: "待检测文件",
                value: viewModel.inputSourceText,
                systemImage: "tray.and.arrow.down",
                onTap: { viewModel.selectFiles() }
            )
            Divider()
                .background(Color.white.opacity(colorScheme == .dark ? 0.2 : 0.4))
            directoryInfoRow(
                title: "检测模式",
                value: "扫描 128-bit 水印消息，解码标签与时间戳",
                systemImage: "doc.text.magnifyingglass",
                onTap: { viewModel.selectFiles() }
            )
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(DesignSystem.Colors.titleBarBackground(colorScheme))
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(
                    colorScheme == .light ? Color.black.opacity(0.08) : Color.clear,
                    lineWidth: 1
                )
        )
    }

    private func directoryInfoRow(title: String, value: String, systemImage: String, onTap: @escaping () -> Void) -> some View {
        HoverBackground {
            Button(action: onTap) {
                HStack(alignment: .center, spacing: 12) {
                    Image(systemName: systemImage)
                        .font(.title3)
                        .foregroundColor(.accentColor)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(title.uppercased())
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(value)
                            .font(.callout)
                            .foregroundColor(.primary)
                            .lineLimit(2)
                            .truncationMode(.middle)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .foregroundColor(.secondary)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
    }

    // MARK: - 操作卡片（右上）

    private var operationCard: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("检测信息")
                    .font(.headline)
                Spacer()
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(DesignSystem.Colors.titleBarBackground(colorScheme))
            .cornerRadius(10)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(
                        colorScheme == .light ? Color.black.opacity(0.08) : Color.clear,
                        lineWidth: 1
                    )
            )

            Divider().opacity(0.5)

            VStack(alignment: .leading, spacing: 8) {
                Text("水印检测会扫描音频文件中嵌入的 128-bit 消息，解码后获取标签身份和嵌入时间戳。")
                    .font(.subheadline)
                    .foregroundColor(.secondary)

                Text("支持格式: WAV, FLAC")
                    .font(.caption)
                    .foregroundColor(.secondary)

                Text("检测不会修改原文件。")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            if viewModel.totalDetected > 0 {
                VStack(spacing: DesignSystem.Spacing.compact) {
                    HStack {
                        Text("已检测")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Spacer()
                        Text("\(viewModel.totalDetected) 个文件")
                            .font(.system(.subheadline, design: .monospaced))
                    }
                    .entryRowStyle()

                    HStack {
                        Text("发现水印")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Spacer()
                        Text("\(viewModel.totalFound) 个")
                            .font(.system(.subheadline, design: .monospaced))
                            .foregroundStyle(viewModel.totalFound > 0 ? DesignSystem.Colors.success : .primary)
                    }
                    .entryRowStyle()
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
    }

    // MARK: - 文件列表卡片（左下）

    private var fileListCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("待检测文件")
                        .font(.headline)
                    Spacer()
                    if !viewModel.selectedFiles.isEmpty {
                        Text("共 \(viewModel.selectedFiles.count) 个")
                            .foregroundColor(.secondary)
                            .font(.subheadline)
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                .background(DesignSystem.Colors.titleBarBackground(colorScheme))
                .cornerRadius(10)
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(
                            colorScheme == .light ? Color.black.opacity(0.08) : Color.clear,
                            lineWidth: 1
                        )
                )

                HStack(spacing: 8) {
                    LiquidGlassProgressView(progress: viewModel.progress, height: 8)
                    Text("\(Int(viewModel.progress * 100))%")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .monospacedDigit()
                        .frame(width: 36, alignment: .trailing)
                }

                if viewModel.selectedFiles.isEmpty {
                    Text("暂无文件")
                        .foregroundColor(.secondary)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                        .transition(.opacity)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 12) {
                            ForEach(Array(viewModel.selectedFiles.enumerated()), id: \.offset) { index, url in
                                detectFileEntryRow(url: url, index: index)
                                    .transition(.asymmetric(
                                        insertion: .move(edge: .leading).combined(with: .opacity),
                                        removal: .move(edge: .leading).combined(with: .opacity)
                                    ))
                            }
                        }
                        .padding(.vertical, 4)
                    }
                    .scrollIndicators(.hidden)
                    .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.35), value: viewModel.selectedFiles.count)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .id(colorScheme)
    }

    private func detectFileEntryRow(url: URL, index: Int) -> some View {
        let (statusText, isActive) = viewModel.fileStatusText(for: url, at: index)

        return FileEntryRow(
            name: url.lastPathComponent,
            detail: url.deletingLastPathComponent().path(percentEncoded: false),
            statusText: statusText,
            isProcessing: isActive
        )
    }

    // MARK: - 事件日志卡片（右下）

    private var logCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("检测日志")
                        .font(.headline)
                    Spacer()
                    if !viewModel.logs.isEmpty {
                        Text("共 \(viewModel.logs.count) 条")
                            .foregroundColor(.secondary)
                            .font(.subheadline)
                    }
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(DesignSystem.Colors.titleBarBackground(colorScheme))
                .cornerRadius(10)
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(
                            colorScheme == .light ? Color.black.opacity(0.08) : Color.clear,
                            lineWidth: 1
                        )
                )

                Divider().opacity(0.5)

                if viewModel.logs.isEmpty {
                    Text("暂无日志")
                        .foregroundColor(.secondary)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                        .transition(.opacity)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 12) {
                            ForEach(viewModel.logs) { entry in
                                logEntryRow(entry: entry)
                                    .transition(.asymmetric(
                                        insertion: .move(edge: .top).combined(with: .opacity),
                                        removal: .move(edge: .top).combined(with: .opacity)
                                    ))
                                    .onAppear {
                                        handleEphemeralEntry(entry)
                                    }
                            }
                        }
                        .padding(.vertical, 4)
                    }
                    .scrollIndicators(.hidden)
                    .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.3), value: viewModel.logs.count)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private func logEntryRow(entry: LogEntry) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top) {
                Image(systemName: entry.isSuccess ? "checkmark.seal.fill" : "xmark.seal.fill")
                    .foregroundStyle(entry.isSuccess ? DesignSystem.Colors.success : Color.secondary)

                if colorScheme == .dark {
                    HDRGlowText(
                        text: entry.title,
                        font: .subheadline.weight(.semibold),
                        intensity: .medium
                    )
                    .lineLimit(1)
                } else {
                    Text(entry.title)
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                }

                Spacer()

                Text(entry.timestamp, format: Date.FormatStyle()
                    .hour().minute().second())
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            if !entry.detail.isEmpty {
                Text(entry.detail)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .lineLimit(2)
                    .truncationMode(.middle)
            }
        }
        .entryRowStyle()
    }

    private func handleEphemeralEntry(_ entry: LogEntry) {
        guard entry.isEphemeral, entry.title == "已清空日志" else { return }
        Task {
            try? await Task.sleep(for: .seconds(3))
            withAnimation(.easeInOut(duration: 0.3)) {
                viewModel.logs.removeAll(where: { $0.id == entry.id })
            }
        }
    }
}
