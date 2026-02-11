import SwiftUI
import UniformTypeIdentifiers

struct DetectView: View {
    @EnvironmentObject var appState: AppState
    @ObservedObject var viewModel: DetectViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var isDropTargeted = false
    @State private var selectedResultLogID: UUID?
    @State private var hideDetectDetailWhenNoSelection = false
    @State private var logSearchText = ""

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
        .onChange(of: viewModel.logs) { _, logs in
            if logs.isEmpty {
                selectedResultLogID = nil
                hideDetectDetailWhenNoSelection = false
                logSearchText = ""
                return
            }
            if let selectedResultLogID,
               !logs.contains(where: { $0.id == selectedResultLogID }) {
                self.selectedResultLogID = nil
                hideDetectDetailWhenNoSelection = false
            }
        }
        .onChange(of: viewModel.detectRecords.count) { oldCount, newCount in
            if newCount == 0 {
                hideDetectDetailWhenNoSelection = false
                return
            }
            if newCount > oldCount {
                hideDetectDetailWhenNoSelection = false
            }
        }
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
                    .disabled(viewModel.selectedFiles.isEmpty || viewModel.isProcessing || !appState.keyLoaded)

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
                if viewModel.totalDetected > 0 {
                    Text("\(viewModel.totalFound)（成功）/\(viewModel.totalDetected)（总）")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(viewModel.totalFound > 0 ? DesignSystem.Colors.success : .secondary)
                        .monospacedDigit()
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

            VStack(spacing: 8) {
                singleLineFieldRow(
                    label: "文件",
                    value: detailValue(from: displayedDetectRecord?.file),
                    valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                    truncationMode: .middle
                )

                tripleFieldRow(
                    (
                        label: "状态",
                        value: detailValue(from: displayedDetectRecord?.status),
                        valueColor: fieldValueColor(for: .status, record: displayedDetectRecord),
                        monospaced: true
                    ),
                    (
                        label: "匹配标记",
                        value: detailValue(from: displayedDetectRecord?.matchFound.map { $0 ? "true" : "false" }),
                        valueColor: fieldValueColor(for: .matchFound, record: displayedDetectRecord),
                        monospaced: true
                    ),
                    (
                        label: "检测模式",
                        value: detailValue(from: displayedDetectRecord?.pattern),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true
                    )
                )

                tripleFieldRow(
                    (
                        label: "标签",
                        value: detailValue(from: displayedDetectRecord?.tag),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true
                    ),
                    (
                        label: "身份",
                        value: detailValue(from: displayedDetectRecord?.identity),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true
                    ),
                    (
                        label: "版本",
                        value: detailValue(from: displayedDetectRecord?.version.map { String($0) }),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true
                    )
                )

                tripleFieldRow(
                    (
                        label: "检测时间",
                        value: localTimestampDisplay(from: displayedDetectRecord),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: localTimestampHelp(from: displayedDetectRecord)
                    ),
                    (
                        label: "密钥槽位",
                        value: detailValue(from: displayedDetectRecord?.keySlot.map { String($0) }),
                        valueColor: fieldValueColor(for: .generic, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: nil
                    ),
                    (
                        label: "位错误",
                        value: detailValue(from: displayedDetectRecord?.bitErrors.map { String($0) }),
                        valueColor: fieldValueColor(for: .bitErrors, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: nil
                    )
                )

                tripleFieldRow(
                    (
                        label: "检测分数",
                        value: detectScoreDisplay(from: displayedDetectRecord),
                        valueColor: fieldValueColor(for: .detectScore, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: detectScoreHelp(from: displayedDetectRecord)
                    ),
                    (
                        label: "克隆校验",
                        value: detailValue(from: displayedDetectRecord?.cloneCheck),
                        valueColor: fieldValueColor(for: .cloneCheck, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: displayedDetectRecord?.cloneReason
                    ),
                    (
                        label: "指纹分数",
                        value: fingerprintScoreDisplay(from: displayedDetectRecord),
                        valueColor: fieldValueColor(for: .fingerprintScore, record: displayedDetectRecord),
                        monospaced: true,
                        helpText: fingerprintScoreHelp(from: displayedDetectRecord)
                    )
                )

                singleLineFieldRow(
                    label: "错误信息",
                    value: errorDisplayValue(from: displayedDetectRecord),
                    valueColor: fieldValueColor(for: .error, record: displayedDetectRecord),
                    truncationMode: .middle
                )
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
                    logSearchField

                    if filteredLogs.isEmpty {
                        Text("无匹配日志")
                            .foregroundColor(.secondary)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                            .transition(.opacity)
                    } else {
                        ScrollView {
                            LazyVStack(spacing: 12) {
                                ForEach(filteredLogs) { entry in
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
            }
            .animation(.easeInOut(duration: 0.3), value: viewModel.logs.count)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    @ViewBuilder
    private func logEntryRow(entry: LogEntry) -> some View {
        let selectable = isSelectableResultLog(entry)
        let isSelected = selectedResultLogID == entry.id

        if selectable {
            Button {
                if isSelected {
                    selectedResultLogID = nil
                    hideDetectDetailWhenNoSelection = true
                } else {
                    selectedResultLogID = entry.id
                    hideDetectDetailWhenNoSelection = false
                }
            } label: {
                logEntryContent(entry: entry, isSelectable: true, isSelected: isSelected)
            }
            .buttonStyle(.plain)
        } else {
            logEntryContent(entry: entry, isSelectable: false, isSelected: false)
        }
    }

    private func logEntryContent(entry: LogEntry, isSelectable: Bool, isSelected: Bool) -> some View {
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
        .overlay {
            if isSelected {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous)
                    .stroke(Color.accentColor, lineWidth: 1.5)
            }
        }
        .background {
            if isSelected {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous)
                    .fill(Color.accentColor.opacity(colorScheme == .dark ? 0.12 : 0.08))
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous))
        .accessibilityLabel(
            isSelectable
                ? "结果日志，可选中查看详情"
                : "流程日志"
        )
    }

    private var logSearchField: some View {
        GlassEffectContainer {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField("搜索日志（标题/详情）", text: $logSearchText)
                    .textFieldStyle(.plain)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
        }
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(
                    DesignSystem.Colors.border(colorScheme),
                    lineWidth: DesignSystem.BorderWidth.standard
                )
        )
    }

    private var filteredLogs: [LogEntry] {
        let query = logSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return viewModel.logs }

        return viewModel.logs.filter { entry in
            entry.title.localizedCaseInsensitiveContains(query) ||
            entry.detail.localizedCaseInsensitiveContains(query)
        }
    }

    private var displayedDetectRecord: DetectRecord? {
        if let selectedResultLogID,
           let selectedLog = viewModel.logs.first(where: { $0.id == selectedResultLogID }),
           let relatedRecordId = selectedLog.relatedRecordId,
           let selectedRecord = viewModel.detectRecords.first(where: { $0.id == relatedRecordId }) {
            return selectedRecord
        }
        if hideDetectDetailWhenNoSelection {
            return nil
        }
        return viewModel.detectRecords.first
    }

    private enum FieldSemantic {
        case generic
        case status
        case matchFound
        case bitErrors
        case detectScore
        case cloneCheck
        case fingerprintScore
        case error
    }

    private var fieldKeyColor: Color {
        colorScheme == .dark
            ? Color.primary.opacity(0.78)
            : Color.primary.opacity(0.68)
    }

    private func singleLineFieldRow(
        label: String,
        value: String,
        valueColor: Color,
        monospaced: Bool = false,
        truncationMode: Text.TruncationMode = .tail
    ) -> some View {
        HStack(alignment: .center, spacing: 10) {
            Text(label)
                .font(.caption.weight(.semibold))
                .foregroundStyle(fieldKeyColor)
                .frame(width: 84, alignment: .leading)

            valueText(
                value,
                color: valueColor,
                monospaced: monospaced,
                truncationMode: truncationMode
            )
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(
                    DesignSystem.Colors.border(colorScheme),
                    lineWidth: DesignSystem.BorderWidth.standard
                )
        )
    }

    private func tripleFieldRow(
        _ first: (label: String, value: String, valueColor: Color, monospaced: Bool),
        _ second: (label: String, value: String, valueColor: Color, monospaced: Bool),
        _ third: (label: String, value: String, valueColor: Color, monospaced: Bool)
    ) -> some View {
        tripleFieldRow(
            (
                label: first.label,
                value: first.value,
                valueColor: first.valueColor,
                monospaced: first.monospaced,
                helpText: nil
            ),
            (
                label: second.label,
                value: second.value,
                valueColor: second.valueColor,
                monospaced: second.monospaced,
                helpText: nil
            ),
            (
                label: third.label,
                value: third.value,
                valueColor: third.valueColor,
                monospaced: third.monospaced,
                helpText: nil
            )
        )
    }

    private func tripleFieldRow(
        _ first: (label: String, value: String, valueColor: Color, monospaced: Bool, helpText: String?),
        _ second: (label: String, value: String, valueColor: Color, monospaced: Bool, helpText: String?),
        _ third: (label: String, value: String, valueColor: Color, monospaced: Bool, helpText: String?)
    ) -> some View {
        HStack(spacing: 8) {
            compactFieldCell(
                label: first.label,
                value: first.value,
                valueColor: first.valueColor,
                monospaced: first.monospaced,
                helpText: first.helpText
            )
            compactFieldCell(
                label: second.label,
                value: second.value,
                valueColor: second.valueColor,
                monospaced: second.monospaced,
                helpText: second.helpText
            )
            compactFieldCell(
                label: third.label,
                value: third.value,
                valueColor: third.valueColor,
                monospaced: third.monospaced,
                helpText: third.helpText
            )
        }
    }

    private func compactFieldCell(
        label: String,
        value: String,
        valueColor: Color,
        monospaced: Bool,
        helpText: String? = nil
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption.weight(.semibold))
                .foregroundStyle(fieldKeyColor)

            valueText(
                value,
                color: valueColor,
                monospaced: monospaced,
                truncationMode: .tail,
                helpText: helpText
            )
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(
                    DesignSystem.Colors.border(colorScheme),
                    lineWidth: DesignSystem.BorderWidth.standard
                )
        )
    }

    private func valueText(
        _ value: String,
        color: Color,
        monospaced: Bool,
        truncationMode: Text.TruncationMode,
        helpText: String? = nil
    ) -> some View {
        let resolvedColor: Color = value == "-" ? .secondary : color

        return Group {
            if monospaced {
                Text(value)
                    .font(.system(.subheadline, design: .monospaced))
            } else {
                Text(value)
                    .font(.subheadline)
            }
        }
        .foregroundStyle(resolvedColor)
        .lineLimit(1)
        .truncationMode(truncationMode)
        .frame(maxWidth: .infinity, alignment: .leading)
        .help(helpText ?? value)
    }

    private func fieldValueColor(for semantic: FieldSemantic, record: DetectRecord?) -> Color {
        switch semantic {
        case .generic:
            return .primary
        case .status:
            guard let status = record?.status else { return .secondary }
            switch status {
            case "ok":
                return DesignSystem.Colors.success
            case "not_found":
                return .secondary
            case "invalid_hmac":
                return DesignSystem.Colors.warning
            case "error":
                return DesignSystem.Colors.error
            default:
                return .secondary
            }
        case .matchFound:
            guard let found = record?.matchFound else { return .secondary }
            return found ? DesignSystem.Colors.success : .secondary
        case .bitErrors:
            guard let bitErrors = record?.bitErrors else { return .secondary }
            if bitErrors == 0 {
                return DesignSystem.Colors.success
            }
            if bitErrors <= 3 {
                return DesignSystem.Colors.warning
            }
            return DesignSystem.Colors.error
        case .detectScore:
            guard let score = record?.detectScore else { return .secondary }
            if score >= 1.30 {
                return DesignSystem.Colors.success
            }
            if score >= 1.10 {
                return DesignSystem.Colors.warning
            }
            if score >= 1.00 {
                return Color.yellow
            }
            return DesignSystem.Colors.error
        case .cloneCheck:
            switch record?.cloneCheck {
            case "exact":
                return DesignSystem.Colors.success
            case "likely":
                return .blue
            case "suspect":
                return DesignSystem.Colors.warning
            case "unavailable":
                return .secondary
            default:
                return .secondary
            }
        case .fingerprintScore:
            guard let score = record?.cloneScore else { return .secondary }
            if score <= 1.0 {
                return DesignSystem.Colors.success
            }
            if score <= 3.0 {
                return .blue
            }
            if score <= 7.0 {
                return DesignSystem.Colors.warning
            }
            return DesignSystem.Colors.error
        case .error:
            let value = errorDisplayValue(from: record)
            return value == "-" ? .secondary : DesignSystem.Colors.error
        }
    }

    private func detailValue(from raw: String?) -> String {
        guard let raw, !raw.isEmpty else { return "-" }
        return raw
    }

    private func detectScoreDisplay(from record: DetectRecord?) -> String {
        guard let score = record?.detectScore else { return "-" }
        return String(format: "%.3f", score)
    }

    private func detectScoreHelp(from record: DetectRecord?) -> String {
        guard let score = record?.detectScore else { return "-" }
        return "检测分数: \(String(format: "%.3f", score))"
    }

    private func fingerprintScoreDisplay(from record: DetectRecord?) -> String {
        guard let score = record?.cloneScore else { return "-" }
        if let seconds = record?.cloneMatchSeconds {
            return "\(String(format: "%.2f", score)) / \(String(format: "%.0f", seconds))s"
        }
        return String(format: "%.2f", score)
    }

    private func fingerprintScoreHelp(from record: DetectRecord?) -> String {
        guard let score = record?.cloneScore else { return "-" }
        if let seconds = record?.cloneMatchSeconds {
            return "指纹分数: \(String(format: "%.3f", score))\n匹配时长: \(String(format: "%.2f", seconds))s"
        }
        return "指纹分数: \(String(format: "%.3f", score))"
    }

    private func errorDisplayValue(from record: DetectRecord?) -> String {
        if let error = record?.error, !error.isEmpty {
            return error
        }
        if let reason = record?.cloneReason, !reason.isEmpty {
            return "clone: \(reason)"
        }
        return "-"
    }

    private func localTimestampDisplay(from record: DetectRecord?) -> String {
        guard let components = timestampComponents(from: record) else {
            return "-"
        }
        let date = Date(timeIntervalSince1970: TimeInterval(components.utcSeconds))
        return Self.localTimestampFormatter.string(from: date)
    }

    private func localTimestampHelp(from record: DetectRecord?) -> String {
        guard let components = timestampComponents(from: record) else {
            return "-"
        }
        let local = localTimestampDisplay(from: record)
        return "本地时间: \(local)\nUTC 分钟: \(components.utcMinutes)\nUTC 秒: \(components.utcSeconds)"
    }

    private func timestampComponents(from record: DetectRecord?) -> (utcMinutes: UInt32, utcSeconds: UInt64)? {
        guard let record else { return nil }
        if let minutes = record.timestampMinutes {
            let seconds = record.timestampUTC ?? UInt64(minutes) * 60
            return (minutes, seconds)
        }
        if let seconds = record.timestampUTC {
            let minutes = UInt32(seconds / 60)
            return (minutes, seconds)
        }
        return nil
    }

    private static let localTimestampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = .autoupdatingCurrent
        formatter.timeZone = .autoupdatingCurrent
        formatter.dateFormat = "yyyy-MM-dd HH:mm"
        return formatter
    }()

    private func isSelectableResultLog(_ entry: LogEntry) -> Bool {
        entry.relatedRecordId != nil
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
            Text("未配置密钥，请前往密钥页完成生成。")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer(minLength: 0)
            Button("前往密钥页") {
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
