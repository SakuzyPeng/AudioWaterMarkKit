import SwiftUI
import UniformTypeIdentifiers

struct EmbedView: View {
    @EnvironmentObject var appState: AppState
    @ObservedObject var viewModel: EmbedViewModel
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
                        settingsCard
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

                    Button(action: { viewModel.selectOutputDirectory() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "externaldrive")
                            Text("选择")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel("选择输出目录")
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.embedFiles(audio: appState.audio) }) {
                        HStack(spacing: 6) {
                            Image(systemName: viewModel.isProcessing ? "stop.fill" : "play.fill")
                                .foregroundColor(viewModel.isProcessing ? .red : .accentColor)
                            Text(viewModel.isProcessing ? "停止" : "嵌入")
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(accentOn: !viewModel.isProcessing, size: .compact))
                    .accessibilityLabel(viewModel.isProcessing ? "停止嵌入" : "开始嵌入")
                    .disabled(viewModel.isCancelling)

                    Button(action: { viewModel.clearQueue() }) {
                        HStack(spacing: 6) {
                            Image(systemName: viewModel.isClearQueueSuccess ? "checkmark.circle" : "xmark.circle")
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
                            Image(systemName: viewModel.isClearLogsSuccess ? "checkmark.circle" : "line.3.horizontal.decrease.circle")
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
                        Image(systemName: "waveform.and.person.filled")
                            .font(.title2)
                        Text("拖拽音频文件到此处")
                            .font(.headline)
                        Text("支持 WAV / FLAC 格式，可批量拖入")
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
                title: "输入文件",
                value: viewModel.inputSourceText,
                systemImage: "tray.and.arrow.down",
                onTap: { viewModel.selectFiles() }
            )
            Divider()
                .background(Color.white.opacity(colorScheme == .dark ? 0.2 : 0.4))
            directoryInfoRow(
                title: "输出目录",
                value: viewModel.outputDirectoryText,
                systemImage: "externaldrive",
                onTap: { viewModel.selectOutputDirectory() }
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

    // MARK: - 设置卡片（右上）

    private var settingsCard: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("嵌入设置")
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

            VStack(alignment: .leading, spacing: 6) {
                Text("标签 (Tag)")
                    .font(.subheadline)
                GlassEffectContainer {
                    TextField("例如: SAKUZY", text: $viewModel.tagInput)
                        .textFieldStyle(.plain)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                }
                .background(DesignSystem.Colors.rowBackground(colorScheme))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(
                            colorScheme == .light ? Color.black.opacity(0.2) : Color.white.opacity(0.25),
                            lineWidth: 1
                        )
                )
                Text("7 字符身份标签，用于追溯水印来源")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text("水印强度")
                        .font(.subheadline)
                    Spacer()
                    Text("\(Int(viewModel.strength))")
                        .font(.system(.body, design: .monospaced))
                }
                Slider(value: $viewModel.strength, in: 1...30, step: 1)
                Text("推荐 10，越高越稳但音质损失越大")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("输出后缀")
                    .font(.subheadline)
                GlassEffectContainer {
                    TextField("_wm", text: $viewModel.customSuffix)
                        .textFieldStyle(.plain)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                }
                .background(DesignSystem.Colors.rowBackground(colorScheme))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(
                            colorScheme == .light ? Color.black.opacity(0.2) : Color.white.opacity(0.25),
                            lineWidth: 1
                        )
                )
                let effective = viewModel.customSuffix.isEmpty ? "_wm" : viewModel.customSuffix
                Text("示例: audio\(effective).wav")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    // MARK: - 文件列表卡片（左下）

    private var fileListCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("待处理文件")
                        .font(.headline)
                    Spacer()
                    if !viewModel.selectedFiles.isEmpty {
                        Text(viewModel.isProcessing ? "剩 \(viewModel.selectedFiles.count) 个" : "共 \(viewModel.selectedFiles.count) 个")
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
                                fileEntryRow(url: url, index: index)
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

    private func fileEntryRow(url: URL, index: Int) -> some View {
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
                    Text("事件 / 日志")
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
                Image(systemName: entry.isSuccess ? "checkmark.circle.fill" : "xmark.circle.fill")
                    .foregroundStyle(entry.isSuccess ? DesignSystem.Colors.success : DesignSystem.Colors.error)

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
        guard entry.isEphemeral else { return }
        Task {
            try? await Task.sleep(for: .seconds(3))
            withAnimation(.easeInOut(duration: 0.3)) {
                viewModel.logs.removeAll(where: { $0.id == entry.id })
            }
        }
    }
}
