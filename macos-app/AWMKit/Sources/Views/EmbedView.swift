import SwiftUI
import UniformTypeIdentifiers

struct EmbedView: View {
    @EnvironmentObject var appState: AppState
    @ObservedObject var viewModel: EmbedViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var isDropTargeted = false
    @State private var showSkipSummaryAlert = false

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
        .onChange(of: viewModel.isProcessing) { oldValue, newValue in
            if oldValue && !newValue {
                Task { await appState.refreshRuntimeStatus() }
            }
        }
        .onChange(of: viewModel.skipSummaryPromptVersion) { _, _ in
            guard viewModel.skipSummaryCount > 0 else { return }
            showSkipSummaryAlert = true
        }
        .alert(
            l("已跳过含水印文件", "Skipped watermarked files"),
            isPresented: $showSkipSummaryAlert
        ) {
            Button(l("我知道了", "OK"), role: .cancel) {}
        } message: {
            Text(viewModel.skipSummaryMessage + "\n" + l("该类文件已自动跳过。", "These files were skipped automatically."))
        }
    }

    private func l(_ zh: String, _ en: String) -> String {
        appState.tr(zh, en)
    }

    // MARK: - 输入卡片（左上）

    private var inputCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 14) {
                directorySummary

                if !appState.keyLoaded {
                    keyRequiredHint
                }

                HStack(spacing: 12) {
                    Button(action: { viewModel.clearInputSource() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "folder")
                            Text(l("清空", "Clear"))
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel(l("清空输入源地址", "Clear input source path"))
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.clearOutputDirectory() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "externaldrive")
                            Text(l("清空", "Clear"))
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel(l("清空输出目录地址", "Clear output directory path"))
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.embedFiles(audio: appState.audio) }) {
                        HStack(spacing: 6) {
                            Image(systemName: viewModel.isProcessing ? "stop.fill" : "play.fill")
                                .foregroundColor(viewModel.isProcessing ? .red : .accentColor)
                            Text(viewModel.isProcessing ? l("停止", "Stop") : l("嵌入", "Embed"))
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(accentOn: !viewModel.isProcessing, size: .compact))
                    .accessibilityLabel(viewModel.isProcessing ? l("停止嵌入", "Stop embedding") : l("开始嵌入", "Start embedding"))
                    .disabled(viewModel.isCancelling || !appState.keyLoaded)

                    Button(action: { viewModel.clearQueue() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "xmark.circle")
                                .foregroundColor(viewModel.isClearQueueSuccess ? .green : .primary)
                            Text(l("清空", "Clear"))
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel(l("清空队列", "Clear queue"))
                    .disabled(viewModel.isProcessing)

                    Button(action: { viewModel.clearLogs() }) {
                        HStack(spacing: 6) {
                            Image(systemName: "line.3.horizontal.decrease.circle")
                                .foregroundColor(viewModel.isClearLogsSuccess ? .green : .primary)
                            Text(l("清空", "Clear"))
                                .lineLimit(1)
                        }
                    }
                    .buttonStyle(GlassButtonStyle(size: .compact))
                    .accessibilityLabel(l("清空日志", "Clear logs"))
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
                        Text(l("拖拽音频文件到此处", "Drop audio files here"))
                            .font(.headline)
                        Text(l("支持 WAV / FLAC / M4A / ALAC 格式，可批量拖入", "Supports WAV / FLAC / M4A / ALAC, batch drop enabled"))
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
                title: l("输入文件", "Input source"),
                value: viewModel.inputSourceText,
                systemImage: "tray.and.arrow.down",
                onTap: { viewModel.selectFiles() }
            )
            Divider()
                .background(Color.white.opacity(colorScheme == .dark ? 0.2 : 0.4))
            directoryInfoRow(
                title: l("输出目录", "Output directory"),
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
                Text(l("嵌入设置", "Embed settings"))
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
                HStack {
                    Text(l("用户名", "Username"))
                        .font(.subheadline)
                    Spacer()
                    HStack(spacing: 8) {
                        if let hint = viewModel.matchedMappingHintText {
                            Text(hint)
                                .font(.caption2)
                                .foregroundStyle(.green)
                                .lineLimit(1)
                                .truncationMode(.tail)
                        }
                        Text("Tag: \(viewModel.previewTagText)")
                            .font(.system(.caption, design: .monospaced).weight(.semibold))
                            .foregroundStyle(
                                viewModel.matchedMappingHintText != nil
                                    ? Color.green
                                    : (viewModel.previewTagText == "-" ? .secondary : .primary)
                            )
                    }
                }
                GlassEffectContainer {
                    TextField(l("例如: user_001", "e.g. user_001"), text: $viewModel.usernameInput)
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
                if viewModel.hasMappingSuggestions {
                    Menu {
                        ForEach(viewModel.mappingSuggestions, id: \.username) { option in
                            Button {
                                viewModel.selectMapping(option)
                            } label: {
                                Text(
                                    "\(Text(option.username).fontWeight(.semibold))\(Text("（\(option.tag)）").font(.system(.caption, design: .monospaced)).foregroundStyle(.secondary))"
                                )
                                .lineLimit(1)
                            }
                        }
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "list.bullet")
                            Text(l("已存储的映射", "Stored mappings"))
                                .lineLimit(1)
                            Spacer()
                            Image(systemName: "chevron.down")
                                .font(.caption2)
                        }
                        .font(.caption)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                        .frame(maxWidth: .infinity, minHeight: 30, alignment: .leading)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .background(DesignSystem.Colors.rowBackground(colorScheme))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(
                                colorScheme == .light ? Color.black.opacity(0.14) : Color.white.opacity(0.18),
                                lineWidth: 1
                            )
                    )
                }
                Text(l("按用户名稳定生成 Tag；命中映射时优先使用已保存 Tag", "Generate stable Tag from username; reuse saved mapping when matched"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(l("水印强度", "Watermark strength"))
                        .font(.subheadline)
                    Spacer()
                    Text("\(Int(viewModel.strength))")
                        .font(.system(.body, design: .monospaced))
                }
                Slider(value: $viewModel.strength, in: 1...30, step: 1)
                Text(l("推荐 10，越高越稳但音质损失越大", "Recommended 10; higher is more robust but degrades audio more"))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text(l("输出后缀", "Output suffix"))
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
                Text("\(l("示例", "Example")): audio\(effective).wav")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            viewModel.refreshTagMappings()
        }
    }

    // MARK: - 文件列表卡片（左下）

    private var fileListCard: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text(l("待处理文件", "Pending files"))
                        .font(.headline)
                    Spacer()
                    if !viewModel.selectedFiles.isEmpty {
                        Text(viewModel.isProcessing
                             ? "\(l("剩", "Left")) \(viewModel.selectedFiles.count)\(l(" 个", ""))"
                             : "\(l("共", "Total")) \(viewModel.selectedFiles.count)\(l(" 个", ""))")
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
                    Text(l("暂无文件", "No files"))
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
                    Text(l("事件 / 日志", "Events / Logs"))
                        .font(.headline)
                    Spacer()
                    if !viewModel.logs.isEmpty {
                        Text("\(l("共", "Total")) \(viewModel.logs.count)\(l(" 条", ""))")
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
                    Text(l("暂无日志", "No logs"))
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
                Image(systemName: entry.iconName)
                    .foregroundStyle(logIconColor(for: entry.iconTone))

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
        guard entry.isEphemeral, entry.kind == .logsCleared else { return }
        Task {
            try? await Task.sleep(for: .seconds(3))
            withAnimation(.easeInOut(duration: 0.3)) {
                viewModel.logs.removeAll(where: { $0.id == entry.id })
            }
        }
    }

    private func logIconColor(for tone: LogEntry.IconTone) -> Color {
        switch tone {
        case .success:
            return DesignSystem.Colors.success
        case .info:
            return .accentColor
        case .warning:
            return DesignSystem.Colors.warning
        case .error:
            return DesignSystem.Colors.error
        }
    }

    private var keyRequiredHint: some View {
        HStack(spacing: 10) {
            Image(systemName: "key.slash")
                .foregroundStyle(DesignSystem.Colors.warning)
            Text(l("未配置密钥，请前往密钥页完成生成。", "No key configured. Open the Key page to generate one."))
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer(minLength: 0)
            Button(l("前往密钥页", "Go to Key page")) {
                appState.selectedTab = .key
            }
            .buttonStyle(GlassButtonStyle(size: .compact))
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(8)
    }
}
