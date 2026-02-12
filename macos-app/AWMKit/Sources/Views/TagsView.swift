import SwiftUI
import AWMKit

struct TagsView: View {
    @EnvironmentObject private var appState: AppState
    @Environment(\.colorScheme) private var colorScheme

    @State private var mappings: [TagMappingEntry] = []
    @State private var evidenceEntries: [EvidenceEntry] = []
    @State private var queryText: String = ""
    @State private var queryScope: QueryScope = .all
    @State private var queryScopeBeforeDeleteMode: QueryScope?

    @State private var newUsername: String = ""
    @State private var showingAddSheet = false

    @State private var deleteMode: DeleteMode = .none
    @State private var selectedUsernames: Set<String> = []
    @State private var selectedEvidenceIDs: Set<Int64> = []
    @State private var showingDeleteConfirm = false
    @State private var deleteConfirmTarget: DeleteTarget = .mappings
    @State private var deleteConfirmInput = ""

    @State private var errorMessage: String?

    private func l(_ zh: String, _ en: String) -> String {
        appState.tr(zh, en)
    }

    private let mappingColumns = [
        GridItem(.adaptive(minimum: 180, maximum: 260), spacing: DesignSystem.Spacing.compact)
    ]

    private enum QueryScope: CaseIterable, Identifiable {
        case all
        case mappings
        case evidence

        var id: Int {
            switch self {
            case .all: return 0
            case .mappings: return 1
            case .evidence: return 2
            }
        }
    }

    private enum DeleteMode {
        case none
        case mappings
        case evidence
    }

    private enum DeleteTarget {
        case mappings
        case evidence
    }

    var body: some View {
        GeometryReader { proxy in
            VStack(spacing: DesignSystem.Spacing.card) {
                toolbarSection

                panelSection
                    .frame(maxHeight: .infinity)

                actionSection

                Spacer(minLength: 0)
            }
            .padding(.horizontal, DesignSystem.Spacing.horizontal)
            .padding(.vertical, DesignSystem.Spacing.vertical)
            .frame(width: proxy.size.width, alignment: .top)
        }
        .sheet(isPresented: $showingAddSheet) {
            AddTagSheet(
                username: $newUsername,
                onSave: saveNewTag
            )
        }
        .sheet(isPresented: $showingDeleteConfirm) {
            DeleteConfirmSheet(
                expectedCount: deleteTargetCount,
                itemLabel: deleteConfirmTarget == .mappings ? l("标签", "mapping") : l("证据", "evidence"),
                input: $deleteConfirmInput,
                onCancel: {
                    showingDeleteConfirm = false
                },
                onConfirm: {
                    showingDeleteConfirm = false
                    performBatchDelete(target: deleteConfirmTarget)
                }
            )
        }
        .alert(l("操作失败", "Operation failed"), isPresented: .init(
            get: { errorMessage != nil },
            set: { isPresented in
                if !isPresented {
                    errorMessage = nil
                }
            }
        )) {
            Button(l("确定", "OK"), role: .cancel) {}
        } message: {
            Text(errorMessage ?? "")
        }
        .onChange(of: mappings.map(\.username)) { _, usernames in
            selectedUsernames = selectedUsernames.intersection(Set(usernames))
            if mappings.isEmpty, deleteMode == .mappings {
                exitDeleteMode()
            }
        }
        .onChange(of: evidenceEntries.map(\.id)) { _, ids in
            selectedEvidenceIDs = selectedEvidenceIDs.intersection(Set(ids))
            if evidenceEntries.isEmpty, deleteMode == .evidence {
                exitDeleteMode()
            }
        }
        .onAppear(perform: loadData)
    }

    private var toolbarSection: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            searchField

            GlassEffectContainer {
                Picker("", selection: $queryScope) {
                    ForEach(QueryScope.allCases) { scope in
                        Text(queryScopeTitle(scope))
                            .tag(scope)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .controlSize(.large)
            }
            .frame(width: 230)
            .disabled(deleteMode != .none)

            Spacer(minLength: 0)

            HStack(spacing: 8) {
                StatusCapsule(
                    status: "\(appState.mappingCount) \(l("映射", "mappings"))",
                    isHighlight: appState.mappingCount > 0
                )
                StatusCapsule(
                    status: "\(appState.evidenceCount) \(l("证据", "evidence"))",
                    isHighlight: appState.evidenceCount > 0
                )
            }
        }
    }

    private var searchField: some View {
        GlassEffectContainer {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)

                TextField(l("搜索用户名 / Tag / Identity / 路径", "Search username / Tag / Identity / path"), text: $queryText)
                    .textFieldStyle(.plain)

                if !queryText.isEmpty {
                    Button {
                        queryText = ""
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                    }
                    .buttonStyle(.plain)
                }
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

    @ViewBuilder
    private var panelSection: some View {
        switch queryScope {
        case .all:
            HStack(spacing: DesignSystem.Spacing.item) {
                mappingPanel
                evidencePanel
            }
        case .mappings:
            mappingPanel
        case .evidence:
            evidencePanel
        }
    }

    private var mappingPanel: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                cardHeader(
                    title: l("标签映射", "Tag mappings"),
                    status: "\(displayedMappings.count)/\(mappings.count)",
                    highlight: !displayedMappings.isEmpty
                )

                if mappings.isEmpty {
                    emptyState(
                        icon: "tag",
                        title: l("暂无标签映射", "No tag mappings"),
                        subtitle: l("点击下方按钮添加用户标签", "Use the button below to add mappings")
                    )
                } else if displayedMappings.isEmpty {
                    emptyState(
                        icon: "magnifyingglass",
                        title: l("未找到匹配映射", "No matched mappings"),
                        subtitle: l("尝试更换关键词", "Try another keyword")
                    )
                } else {
                    ScrollView {
                        LazyVGrid(columns: mappingColumns, spacing: DesignSystem.Spacing.compact) {
                            ForEach(displayedMappings, id: \.username) { entry in
                                TagEntryRow(
                                    entry: entry,
                                    isDeleteMode: deleteMode == .mappings,
                                    isSelected: selectedUsernames.contains(entry.username),
                                    onToggleSelected: {
                                        toggleMappingSelection(username: entry.username)
                                    }
                                )
                            }
                        }
                        .padding(.vertical, 2)
                    }
                    .scrollIndicators(.hidden)
                }
            }
        }
    }

    private var evidencePanel: some View {
        GlassCard {
            VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                cardHeader(
                    title: l("音频证据", "Audio evidence"),
                    status: "\(displayedEvidence.count)/\(evidenceEntries.count)",
                    highlight: !displayedEvidence.isEmpty
                )

                if evidenceEntries.isEmpty {
                    emptyState(
                        icon: "waveform",
                        title: l("暂无证据记录", "No evidence records"),
                        subtitle: l("完成嵌入后会自动写入证据数据库", "Evidence is auto-written after embedding")
                    )
                } else if displayedEvidence.isEmpty {
                    emptyState(
                        icon: "magnifyingglass",
                        title: l("未找到匹配证据", "No matched evidence"),
                        subtitle: l("尝试更换关键词", "Try another keyword")
                    )
                } else {
                    ScrollView {
                        LazyVStack(spacing: DesignSystem.Spacing.compact) {
                            ForEach(displayedEvidence) { entry in
                                EvidenceEntryRow(
                                    entry: entry,
                                    isDeleteMode: deleteMode == .evidence,
                                    isSelected: selectedEvidenceIDs.contains(entry.id),
                                    onToggleSelected: {
                                        toggleEvidenceSelection(id: entry.id)
                                    }
                                )
                            }
                        }
                        .padding(.vertical, 2)
                    }
                    .scrollIndicators(.hidden)
                }
            }
        }
    }

    private func emptyState(icon: String, title: String, subtitle: String) -> some View {
        HStack {
            Spacer()
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.system(size: 28))
                    .foregroundStyle(.secondary)
                Text(title)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            Spacer()
        }
        .frame(minHeight: 140)
    }

    private func cardHeader(title: String, status: String, highlight: Bool) -> some View {
        HStack {
            Text(title)
                .font(.headline.weight(.semibold))

            Spacer()

            StatusCapsule(
                status: status,
                isHighlight: highlight
            )
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
    }

    private var actionSection: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            switch deleteMode {
            case .none:
                Button(action: { showingAddSheet = true }) {
                    HStack {
                        Image(systemName: "plus.circle")
                        Text(l("添加标签", "Add mapping"))
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))

                Button(action: { enterDeleteMode(.mappings) }) {
                    HStack {
                        Image(systemName: "tag")
                        Text(l("删除标签", "Delete mappings"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(mappings.isEmpty)

                Button(action: { enterDeleteMode(.evidence) }) {
                    HStack {
                        Image(systemName: "waveform")
                        Text(l("删除证据", "Delete evidence"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(evidenceEntries.isEmpty)

            case .mappings:
                Button(action: exitDeleteMode) {
                    HStack {
                        Image(systemName: "xmark.circle")
                        Text(l("退出删除", "Exit delete"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .accessibilityLabel(l("退出标签删除模式", "Exit mapping delete mode"))

                Button(action: selectAllMappings) {
                    HStack {
                        Image(systemName: "checkmark.circle")
                        Text(l("全选", "Select all"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(mappings.isEmpty)

                Button(action: clearMappingSelection) {
                    HStack {
                        Image(systemName: "circle.dashed")
                        Text(l("全不选", "Clear selection"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(selectedUsernames.isEmpty)

                Button(action: { handleDeleteAction(target: .mappings) }) {
                    HStack {
                        Image(systemName: "trash")
                        Text(l("执行删除", "Run delete"))
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))

            case .evidence:
                Button(action: exitDeleteMode) {
                    HStack {
                        Image(systemName: "xmark.circle")
                        Text(l("退出删除", "Exit delete"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .accessibilityLabel(l("退出证据删除模式", "Exit evidence delete mode"))

                Button(action: selectAllEvidence) {
                    HStack {
                        Image(systemName: "checkmark.circle")
                        Text(l("全选", "Select all"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(evidenceEntries.isEmpty)

                Button(action: clearEvidenceSelection) {
                    HStack {
                        Image(systemName: "circle.dashed")
                        Text(l("全不选", "Clear selection"))
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(selectedEvidenceIDs.isEmpty)

                Button(action: { handleDeleteAction(target: .evidence) }) {
                    HStack {
                        Image(systemName: "trash")
                        Text(l("执行删除", "Run delete"))
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
            }

            Spacer()
        }
    }

    private var trimmedQuery: String {
        queryText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func queryScopeTitle(_ scope: QueryScope) -> String {
        switch scope {
        case .all:
            return l("全部", "All")
        case .mappings:
            return l("映射", "Mappings")
        case .evidence:
            return l("证据", "Evidence")
        }
    }

    private var displayedMappings: [TagMappingEntry] {
        guard queryScope != .evidence else { return [] }
        guard !trimmedQuery.isEmpty else { return mappings }

        return mappings.filter { entry in
            entry.username.localizedCaseInsensitiveContains(trimmedQuery) ||
            entry.tag.localizedCaseInsensitiveContains(trimmedQuery)
        }
    }

    private var displayedEvidence: [EvidenceEntry] {
        guard queryScope != .mappings else { return [] }
        guard !trimmedQuery.isEmpty else { return evidenceEntries }

        return evidenceEntries.filter { entry in
            entry.identity.localizedCaseInsensitiveContains(trimmedQuery) ||
            entry.tag.localizedCaseInsensitiveContains(trimmedQuery) ||
            (entry.keyId?.localizedCaseInsensitiveContains(trimmedQuery) ?? false) ||
            entry.filePath.localizedCaseInsensitiveContains(trimmedQuery)
        }
    }

    private var deleteTargetCount: Int {
        switch deleteConfirmTarget {
        case .mappings:
            return selectedUsernames.count
        case .evidence:
            return selectedEvidenceIDs.count
        }
    }

    private func loadData() {
        do {
            mappings = try DatabaseQueryStore.listTagMappings()
            evidenceEntries = try DatabaseQueryStore.listEvidence()
            refreshDatabaseStatus()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func saveNewTag() {
        guard !newUsername.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return
        }

        do {
            mappings = try DatabaseQueryStore.saveTagMapping(username: newUsername)
            newUsername = ""
            showingAddSheet = false
            refreshDatabaseStatus()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func enterDeleteMode(_ mode: DeleteMode) {
        if deleteMode == .none {
            queryScopeBeforeDeleteMode = queryScope
        }
        deleteMode = mode
        queryScope = mode == .mappings ? .mappings : .evidence
        selectedUsernames.removeAll()
        selectedEvidenceIDs.removeAll()
        deleteConfirmInput = ""
        showingDeleteConfirm = false
    }

    private func exitDeleteMode() {
        deleteMode = .none
        if let previousScope = queryScopeBeforeDeleteMode {
            queryScope = previousScope
        }
        queryScopeBeforeDeleteMode = nil
        selectedUsernames.removeAll()
        selectedEvidenceIDs.removeAll()
        deleteConfirmInput = ""
        showingDeleteConfirm = false
    }

    private func toggleMappingSelection(username: String) {
        guard deleteMode == .mappings else { return }
        if selectedUsernames.contains(username) {
            selectedUsernames.remove(username)
        } else {
            selectedUsernames.insert(username)
        }
    }

    private func toggleEvidenceSelection(id: Int64) {
        guard deleteMode == .evidence else { return }
        if selectedEvidenceIDs.contains(id) {
            selectedEvidenceIDs.remove(id)
        } else {
            selectedEvidenceIDs.insert(id)
        }
    }

    private func selectAllMappings() {
        selectedUsernames = Set(mappings.map(\.username))
    }

    private func clearMappingSelection() {
        selectedUsernames.removeAll()
    }

    private func selectAllEvidence() {
        selectedEvidenceIDs = Set(evidenceEntries.map(\.id))
    }

    private func clearEvidenceSelection() {
        selectedEvidenceIDs.removeAll()
    }

    private func handleDeleteAction(target: DeleteTarget) {
        switch target {
        case .mappings where selectedUsernames.isEmpty:
            exitDeleteMode()
            return
        case .evidence where selectedEvidenceIDs.isEmpty:
            exitDeleteMode()
            return
        default:
            break
        }

        deleteConfirmTarget = target
        deleteConfirmInput = ""
        DispatchQueue.main.async {
            showingDeleteConfirm = true
        }
    }

    private func performBatchDelete(target: DeleteTarget) {
        do {
            switch target {
            case .mappings:
                mappings = try DatabaseQueryStore.removeTagMappings(usernames: selectedUsernames)
            case .evidence:
                evidenceEntries = try DatabaseQueryStore.removeEvidence(ids: selectedEvidenceIDs)
            }
            exitDeleteMode()
            refreshDatabaseStatus()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func refreshDatabaseStatus() {
        Task { await appState.refreshRuntimeStatus() }
    }
}

private func localizedTagsText(_ zh: String, _ en: String) -> String {
    ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS ? en : zh
}

private struct TagEntryRow: View {
    let entry: TagMappingEntry
    let isDeleteMode: Bool
    let isSelected: Bool
    let onToggleSelected: () -> Void

    var body: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            Image(systemName: "person.circle.fill")
                .foregroundStyle(.secondary)
                .font(.system(size: 17, weight: .medium))

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.username)
                    .font(.subheadline.weight(.medium))
                    .lineLimit(1)

                Text(entry.tag)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            Text(threeLineDateText(Date(timeIntervalSince1970: TimeInterval(entry.createdAt))))
                .font(.caption2)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .lineLimit(3)
                .frame(width: 82, alignment: .center)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .entryRowStyle()
        .overlay {
            if isDeleteMode && isSelected {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous)
                    .stroke(Color.accentColor.opacity(0.9), lineWidth: 1.5)
            }
        }
        .overlay(alignment: .topTrailing) {
            if isDeleteMode && isSelected {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .padding(6)
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous))
        .onTapGesture {
            if isDeleteMode {
                onToggleSelected()
            }
        }
    }
}

private struct EvidenceEntryRow: View {
    let entry: EvidenceEntry
    let isDeleteMode: Bool
    let isSelected: Bool
    let onToggleSelected: () -> Void

    var body: some View {
        let snrText: String = {
            guard entry.snrStatus == "ok", let value = entry.snrDb else { return "" }
            return String(format: " · SNR %.2f dB", value)
        }()

        HStack(spacing: DesignSystem.Spacing.item) {
            Image(systemName: "waveform")
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(.secondary)
                .frame(width: 18, height: 18)

            VStack(alignment: .leading, spacing: 6) {
                Text(entry.identity)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)

                Text("Tag \(entry.tag) · \(localizedTagsText("槽位", "Slot")) \(entry.keySlot) · Key ID \(entry.keyId ?? "-")\(snrText)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .help(entry.keyId ?? "-")

                Text(entry.filePath)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .help(entry.filePath)
            }

            Spacer(minLength: 8)

            Text(threeLineDateText(entry.createdDate))
                .font(.caption2)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .lineLimit(3)
                .frame(width: 82, alignment: .center)
        }
        .entryRowStyle()
        .overlay {
            if isDeleteMode && isSelected {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous)
                    .stroke(Color.accentColor.opacity(0.9), lineWidth: 1.5)
            }
        }
        .overlay(alignment: .topTrailing) {
            if isDeleteMode && isSelected {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .padding(6)
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous))
        .onTapGesture {
            if isDeleteMode {
                onToggleSelected()
            }
        }
    }
}

private func threeLineDateText(_ date: Date) -> String {
    let calendar = Calendar.current
    let components = calendar.dateComponents([.year, .month, .day, .hour, .minute, .second], from: date)
    let year = components.year ?? 0
    let month = components.month ?? 0
    let day = components.day ?? 0
    let hour = components.hour ?? 0
    let minute = components.minute ?? 0
    let second = components.second ?? 0
    let isEnglish = ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS
    if isEnglish {
        return "\(year)\n\(month)/\(day)\n\(String(format: "%02d:%02d:%02d", hour, minute, second))"
    }
    return "\(year)年\n\(month)月\(day)日\n\(String(format: "%02d:%02d:%02d", hour, minute, second))"
}

private struct AddTagSheet: View {
    @Binding var username: String
    let onSave: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var suggestedTag: String? {
        DatabaseQueryStore.previewTag(username: username)
    }

    private func l(_ zh: String, _ en: String) -> String {
        localizedTagsText(zh, en)
    }

    var body: some View {
        VStack(spacing: DesignSystem.Spacing.card) {
            Text(l("添加标签映射", "Add tag mapping"))
                .font(.title2.weight(.semibold))

            VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                VStack(alignment: .leading, spacing: 6) {
                    Text(l("用户名", "Username"))
                        .font(.subheadline)
                        .foregroundStyle(.secondary)

                    GlassEffectContainer {
                        TextField(l("例如: user_001", "e.g. user_001"), text: $username)
                            .textFieldStyle(.plain)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                    }
                    .background(DesignSystem.Colors.rowBackground(colorScheme))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                    )
                }

                VStack(alignment: .leading, spacing: 6) {
                    Text(l("自动生成 Tag", "Auto generated Tag"))
                        .font(.subheadline)
                        .foregroundStyle(.secondary.opacity(0.85))

                    GlassEffectContainer {
                        HStack {
                            Text(suggestedTag ?? "-")
                                .font(.system(.body, design: .monospaced).weight(.semibold))
                                .foregroundStyle(suggestedTag == nil ? .tertiary : .primary)
                            Spacer()
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                    }
                    .background(DesignSystem.Colors.rowBackground(colorScheme))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                    )

                    Text(l("基于用户名稳定生成（预览即最终保存值）", "Stable hash from username (preview is final value)"))
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }

            HStack(spacing: DesignSystem.Spacing.item) {
                Button(l("取消", "Cancel")) {
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button(l("保存", "Save")) {
                    onSave()
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(username.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(30)
        .frame(width: 420)
    }
}

private struct DeleteConfirmSheet: View {
    let expectedCount: Int
    let itemLabel: String
    @Binding var input: String
    let onCancel: () -> Void
    let onConfirm: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var trimmedInput: String {
        input.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var isValid: Bool {
        Int(trimmedInput) == expectedCount
    }

    private func l(_ zh: String, _ en: String) -> String {
        localizedTagsText(zh, en)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DesignSystem.Spacing.card) {
            Text(l("确认删除", "Confirm delete"))
                .font(.title3.weight(.semibold))

            VStack(alignment: .leading, spacing: 8) {
                Text(l("此操作不可恢复，请输入数量确认。", "This operation is irreversible. Please confirm by count."))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Text("\(l("我确认删除", "I confirm deleting")) \(expectedCount) \(itemLabel)")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.primary)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("\(l("请输入数字", "Enter number")): \(expectedCount)")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                GlassEffectContainer {
                    TextField("\(l("输入", "Enter")) \(expectedCount)", text: $input)
                        .textFieldStyle(.plain)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                }
                .background(DesignSystem.Colors.rowBackground(colorScheme))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                )
            }

            HStack(spacing: DesignSystem.Spacing.item) {
                Button(l("取消", "Cancel")) {
                    onCancel()
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button(l("确认删除", "Confirm delete")) {
                    onConfirm()
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(!isValid)
            }
        }
        .padding(24)
        .frame(width: 420)
    }
}
