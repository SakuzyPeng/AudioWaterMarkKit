import SwiftUI

struct TagsView: View {
    @EnvironmentObject private var appState: AppState
    @Environment(\.colorScheme) private var colorScheme

    @State private var mappings: [TagMappingEntry] = []
    @State private var evidenceEntries: [EvidenceEntry] = []
    @State private var queryText: String = ""
    @State private var queryScope: QueryScope = .all

    @State private var newUsername: String = ""
    @State private var showingAddSheet = false

    @State private var deleteMode: DeleteMode = .none
    @State private var selectedUsernames: Set<String> = []
    @State private var selectedEvidenceIDs: Set<Int64> = []
    @State private var showingDeleteConfirm = false
    @State private var deleteConfirmTarget: DeleteTarget = .mappings
    @State private var deleteConfirmInput = ""

    @State private var errorMessage: String?

    private let mappingColumns = [
        GridItem(.adaptive(minimum: 180, maximum: 260), spacing: DesignSystem.Spacing.compact)
    ]

    private enum QueryScope: String, CaseIterable, Identifiable {
        case all = "全部"
        case mappings = "映射"
        case evidence = "证据"

        var id: String { rawValue }
    }

    private enum DeleteMode {
        case none
        case mappings
        case evidence
    }

    private enum DeleteTarget {
        case mappings
        case evidence

        var noun: String {
            switch self {
            case .mappings:
                return "标签"
            case .evidence:
                return "证据"
            }
        }
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
                itemLabel: deleteConfirmTarget.noun,
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
        .alert("操作失败", isPresented: .init(
            get: { errorMessage != nil },
            set: { isPresented in
                if !isPresented {
                    errorMessage = nil
                }
            }
        )) {
            Button("确定", role: .cancel) {}
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
                        Text(scope.rawValue)
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
                    status: "\(appState.mappingCount) 映射",
                    isHighlight: appState.mappingCount > 0
                )
                StatusCapsule(
                    status: "\(appState.evidenceCount) 证据",
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

                TextField("搜索用户名 / Tag / Identity / 路径 / SHA256", text: $queryText)
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
                HStack {
                    Text("标签映射")
                        .font(.headline.weight(.semibold))

                    Spacer()

                    StatusCapsule(
                        status: "\(displayedMappings.count)/\(mappings.count)",
                        isHighlight: !displayedMappings.isEmpty
                    )
                }

                if mappings.isEmpty {
                    emptyState(
                        icon: "tag",
                        title: "暂无标签映射",
                        subtitle: "点击下方按钮添加用户标签"
                    )
                } else if displayedMappings.isEmpty {
                    emptyState(
                        icon: "magnifyingglass",
                        title: "未找到匹配映射",
                        subtitle: "尝试更换关键词"
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
                HStack {
                    Text("音频证据")
                        .font(.headline.weight(.semibold))

                    Spacer()

                    StatusCapsule(
                        status: "\(displayedEvidence.count)/\(evidenceEntries.count)",
                        isHighlight: !displayedEvidence.isEmpty
                    )
                }

                if evidenceEntries.isEmpty {
                    emptyState(
                        icon: "waveform",
                        title: "暂无证据记录",
                        subtitle: "完成嵌入后会自动写入证据数据库"
                    )
                } else if displayedEvidence.isEmpty {
                    emptyState(
                        icon: "magnifyingglass",
                        title: "未找到匹配证据",
                        subtitle: "尝试更换关键词"
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

    private var actionSection: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            switch deleteMode {
            case .none:
                Button(action: { showingAddSheet = true }) {
                    HStack {
                        Image(systemName: "plus.circle")
                        Text("添加标签")
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))

                Button(action: { enterDeleteMode(.mappings) }) {
                    HStack {
                        Image(systemName: "tag")
                        Text("删除标签")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(mappings.isEmpty)

                Button(action: { enterDeleteMode(.evidence) }) {
                    HStack {
                        Image(systemName: "waveform")
                        Text("删除证据")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(evidenceEntries.isEmpty)

            case .mappings:
                Button(action: exitDeleteMode) {
                    HStack {
                        Image(systemName: "xmark.circle")
                        Text("退出删除")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .accessibilityLabel("退出标签删除模式")

                Button(action: selectAllMappings) {
                    HStack {
                        Image(systemName: "checkmark.circle")
                        Text("全选")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(mappings.isEmpty)

                Button(action: clearMappingSelection) {
                    HStack {
                        Image(systemName: "circle.dashed")
                        Text("全不选")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(selectedUsernames.isEmpty)

                Button(action: { handleDeleteAction(target: .mappings) }) {
                    HStack {
                        Image(systemName: "trash")
                        Text("执行删除")
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))

            case .evidence:
                Button(action: exitDeleteMode) {
                    HStack {
                        Image(systemName: "xmark.circle")
                        Text("退出删除")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .accessibilityLabel("退出证据删除模式")

                Button(action: selectAllEvidence) {
                    HStack {
                        Image(systemName: "checkmark.circle")
                        Text("全选")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(evidenceEntries.isEmpty)

                Button(action: clearEvidenceSelection) {
                    HStack {
                        Image(systemName: "circle.dashed")
                        Text("全不选")
                    }
                }
                .buttonStyle(GlassButtonStyle())
                .disabled(selectedEvidenceIDs.isEmpty)

                Button(action: { handleDeleteAction(target: .evidence) }) {
                    HStack {
                        Image(systemName: "trash")
                        Text("执行删除")
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
            entry.filePath.localizedCaseInsensitiveContains(trimmedQuery) ||
            entry.pcmSha256.localizedCaseInsensitiveContains(trimmedQuery)
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
        deleteMode = mode
        queryScope = mode == .mappings ? .mappings : .evidence
        selectedUsernames.removeAll()
        selectedEvidenceIDs.removeAll()
        deleteConfirmInput = ""
        showingDeleteConfirm = false
    }

    private func exitDeleteMode() {
        deleteMode = .none
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
        showingDeleteConfirm = true
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
        appState.checkDatabaseStatus()
    }
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

            Spacer(minLength: 6)
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
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Image(systemName: "waveform.path.badge.shield.checkmark")
                    .foregroundStyle(.secondary)

                Text(entry.identity)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)

                Spacer(minLength: 6)

                Text(entry.createdDate, format: Date.FormatStyle().year().month().day().hour().minute())
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }

            Text("Tag \(entry.tag) · 槽位 \(entry.keySlot) · v\(entry.version)")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)

            Text(entry.filePath)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
                .help(entry.filePath)
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

private struct AddTagSheet: View {
    @Binding var username: String
    let onSave: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var suggestedTag: String? {
        DatabaseQueryStore.previewTag(username: username)
    }

    var body: some View {
        VStack(spacing: DesignSystem.Spacing.card) {
            Text("添加标签映射")
                .font(.title2.weight(.semibold))

            VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("用户名")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)

                    GlassEffectContainer {
                        TextField("例如: user_001", text: $username)
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
                    Text("自动生成 Tag")
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

                    Text("基于用户名稳定生成（预览即最终保存值）")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }

            HStack(spacing: DesignSystem.Spacing.item) {
                Button("取消") {
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button("保存") {
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

    var body: some View {
        VStack(alignment: .leading, spacing: DesignSystem.Spacing.card) {
            Text("确认删除")
                .font(.title3.weight(.semibold))

            VStack(alignment: .leading, spacing: 8) {
                Text("此操作不可恢复，请输入数量确认。")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Text("我确认删除 \(expectedCount) 条\(itemLabel)")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.primary)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("请输入数字：\(expectedCount)")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                GlassEffectContainer {
                    TextField("输入 \(expectedCount)", text: $input)
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
                Button("取消") {
                    onCancel()
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button("确认删除") {
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
