import SwiftUI
import AWMKit

struct KeyView: View {
    @EnvironmentObject private var appState: AppState
    @ObservedObject var viewModel: KeyViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var showDeleteConfirmSheet = false
    @State private var deleteConfirmInput = ""
    @State private var showEditLabelSheet = false
    @State private var editLabelDraft = ""
    @State private var showHexImportSheet = false
    @State private var hexImportDraft = ""

    private func l(_ zh: String, _ en: String) -> String {
        appState.tr(zh, en)
    }

    var body: some View {
        GeometryReader { proxy in
            HStack(alignment: .top, spacing: DesignSystem.Spacing.card) {
                GlassCard {
                    VStack(alignment: .leading, spacing: 14) {
                        header(l("密钥管理", "Key Management"))
                        statusSection
                        slotSection
                        activeKeyCapsule
                        actionSection
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
                .frame(width: max(420, proxy.size.width * 0.42))

                GlassCard {
                    VStack(alignment: .leading, spacing: 12) {
                        header(l("槽位摘要", "Slot Summary"), count: viewModel.configuredSlotCount)
                        slotSearchField
                        slotSummarySection
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
            }
            .padding(.horizontal, DesignSystem.Spacing.horizontal)
            .padding(.vertical, DesignSystem.Spacing.vertical)
            .frame(width: proxy.size.width, height: proxy.size.height, alignment: .top)
        }
        .onAppear {
            viewModel.sync(from: appState)
            Task { await appState.refreshRuntimeStatus() }
        }
        .alert(l("操作结果", "Operation Result"), isPresented: Binding(
            get: { viewModel.errorMessage != nil || viewModel.successMessage != nil },
            set: { newValue in
                if !newValue {
                    viewModel.errorMessage = nil
                    viewModel.successMessage = nil
                }
            })
        ) {
            Button(l("确定", "OK"), role: .cancel) {}
        } message: {
            Text(viewModel.errorMessage ?? viewModel.successMessage ?? "")
        }
        .sheet(isPresented: $showEditLabelSheet) {
            EditSlotLabelSheet(
                slot: appState.activeKeySlot,
                keyId: viewModel.activeSlotSummary?.keyId,
                currentLabel: viewModel.activeSlotSummary?.label,
                draftLabel: $editLabelDraft,
                isWorking: viewModel.isWorking,
                onCancel: { showEditLabelSheet = false },
                onConfirm: {
                    Task {
                        await viewModel.editActiveSlotLabel(appState: appState, label: editLabelDraft)
                        showEditLabelSheet = false
                    }
                }
            )
        }
        .sheet(isPresented: $showDeleteConfirmSheet) {
            KeyDeleteConfirmSheet(
                slot: viewModel.selectedSlot,
                input: $deleteConfirmInput,
                isWorking: viewModel.isWorking,
                onCancel: { showDeleteConfirmSheet = false },
                onConfirm: {
                    Task {
                        await viewModel.deleteKey(appState: appState)
                        showDeleteConfirmSheet = false
                    }
                }
            )
        }
        .sheet(isPresented: $showHexImportSheet) {
            HexKeyImportSheet(
                slot: viewModel.selectedSlot,
                draftHex: $hexImportDraft,
                isWorking: viewModel.isWorking,
                onCancel: { showHexImportSheet = false },
                onConfirm: {
                    Task {
                        await viewModel.importKeyFromHex(appState: appState, hexInput: hexImportDraft)
                        showHexImportSheet = false
                    }
                }
            )
        }
    }

    private func header(_ title: String, count: Int = 0) -> some View {
        HStack {
            Text(title)
                .font(.headline)
            Spacer()
            if count > 0 {
                StatusCapsule(status: "\(count)", isHighlight: true)
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
    }

    private var statusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            row(
                label: l("密钥状态", "Key status"),
                value: appState.keyLoaded ? l("已配置", "Configured") : l("未配置", "Not configured"),
                valueColor: appState.keyLoaded ? DesignSystem.Colors.success : DesignSystem.Colors.warning
            )
            row(label: l("密钥来源", "Key source"), value: appState.keySourceLabel)
        }
    }

    private var slotSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(l("激活槽位", "Active slot"))
                    .font(.subheadline)
                Spacer()
                Picker("", selection: $viewModel.selectedSlot) {
                    ForEach(viewModel.slotOptions, id: \.self) { slot in
                        Text("\(slot)").tag(slot)
                    }
                }
                .labelsHidden()
                .frame(width: 88)
                Button {
                    viewModel.applySlot(appState: appState)
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "square.grid.2x2.fill")
                            .foregroundStyle(viewModel.isApplySuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("应用", "Apply"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
            }

            Text("\(l("当前激活槽位", "Current active slot")): \(appState.activeKeySlot)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private var slotSummarySection: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(viewModel.filteredSlotSummaries, id: \.slot) { summary in
                    slotSummaryRow(summary)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var actionSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Button {
                    Task { await viewModel.generateKey(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "key.fill")
                            .foregroundStyle(viewModel.isGenerateSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("生成", "Generate"))
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(viewModel.isWorking)

                Button {
                    Task { await viewModel.importKeyFromFile(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "square.and.arrow.down")
                            .foregroundStyle(viewModel.isImportSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("导入(.bin)", "Import (.bin)"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.selectedSlotHasKey || viewModel.isWorking)

                Button {
                    hexImportDraft = ""
                    showHexImportSheet = true
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "number.square")
                            .foregroundStyle(viewModel.isHexImportSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("Hex 导入", "Hex Import"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.selectedSlotHasKey || viewModel.isWorking)

                Button {
                    Task { await viewModel.exportKeyToFile(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "square.and.arrow.up")
                            .foregroundStyle(viewModel.isExportSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("导出(.bin)", "Export (.bin)"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(!viewModel.selectedSlotHasKey || viewModel.isWorking)
            }

            HStack(spacing: 10) {
                Button {
                    editLabelDraft = viewModel.activeSlotSummary?.label ?? ""
                    showEditLabelSheet = true
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "tag.fill")
                            .foregroundStyle(viewModel.isEditSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("编辑", "Edit"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.isWorking)

                Button {
                    deleteConfirmInput = ""
                    showDeleteConfirmSheet = true
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "trash")
                            .foregroundStyle(viewModel.isDeleteSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("删除", "Delete"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(!viewModel.selectedSlotHasKey || viewModel.isWorking)

                Button {
                    Task { await viewModel.refresh(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "arrow.clockwise")
                            .foregroundStyle(viewModel.isRefreshSuccess ? DesignSystem.Colors.success : .primary)
                        Text(l("刷新", "Refresh"))
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.isWorking)
            }
        }
    }

    private var slotSearchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
            TextField(l("搜索槽位 / Key ID / 标签 / 状态", "Search slot / Key ID / label / status"), text: $viewModel.slotSearchText)
                .textFieldStyle(.plain)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(10)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(
                    colorScheme == .light ? Color.black.opacity(0.14) : Color.white.opacity(0.18),
                    lineWidth: 1
                )
        )
    }

    private var activeKeyCapsule: some View {
        let summary = viewModel.activeSlotSummary
        let keyText = summary?.hasKey == true ? "Key ID: \(summary?.keyId ?? "-")" : l("未配置", "Not configured")
        let labelText = (summary?.label?.isEmpty == false) ? " · \(summary?.label ?? "")" : ""
        let evidenceText = "\(l("证据", "Evidence")): \(summary?.evidenceCount ?? 0)"
        return VStack(alignment: .leading, spacing: 6) {
            Text(l("当前激活密钥", "Current active key"))
                .font(.subheadline)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Image(systemName: "square.grid.2x2")
                        .foregroundStyle(summary?.isActive == true ? DesignSystem.Colors.success : .secondary)
                    Text("\(l("槽位", "Slot")) \(appState.activeKeySlot)\(summary?.hasKey == true ? l("（已配置）", " (configured)") : l("（未配置）", " (not configured)"))")
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(DesignSystem.Colors.success)
                }
                Text(keyText + labelText)
                    .font(.caption)
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Text(evidenceText)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(DesignSystem.Colors.rowBackground(colorScheme))
            .cornerRadius(8)
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(
                        colorScheme == .light ? Color.black.opacity(0.12) : Color.white.opacity(0.14),
                        lineWidth: 1
                    )
            )
        }
    }

    @ViewBuilder
    private func slotSummaryRow(_ summary: AWMKeySlotSummary) -> some View {
        let title = summary.isActive
            ? "\(l("槽位", "Slot")) \(summary.slot)\(l("（激活）", " (active)"))"
            : "\(l("槽位", "Slot")) \(summary.slot)"
        let keyText = summary.hasKey ? "Key ID: \(summary.keyId ?? "-")" : l("未配置", "Not configured")
        let labelText = summary.label.flatMap { $0.isEmpty ? nil : " · \($0)" } ?? ""
        let evidenceText = "\(l("证据", "Evidence")): \(summary.evidenceCount)"
        let duplicateText = summary.duplicateOfSlots.isEmpty
            ? ""
            : " · \(l("重复", "Duplicate")): \(summary.duplicateOfSlots.map(String.init).joined(separator: ","))"

        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 6) {
                Image(systemName: "square.grid.2x2")
                    .foregroundStyle(summary.isActive ? DesignSystem.Colors.success : .secondary)
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(statusColor(summary.statusText))
            }
            Text(keyText + labelText)
                .font(.caption)
                .foregroundStyle(.primary)
                .lineLimit(1)
                .truncationMode(.tail)
            Text(evidenceText + duplicateText)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.tail)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(DesignSystem.Colors.rowBackground(colorScheme))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(
                    colorScheme == .light ? Color.black.opacity(0.12) : Color.white.opacity(0.14),
                    lineWidth: 1
                )
        )
    }

    private func statusColor(_ status: String) -> Color {
        switch status {
        case "active":
            return .green
        case "duplicate":
            return .orange
        case "configured":
            return .primary
        default:
            return .secondary
        }
    }

    private func row(label: String, value: String, valueColor: Color = .primary) -> some View {
        HStack(spacing: 12) {
            Text(label)
                .foregroundStyle(.secondary)
                .frame(width: 70, alignment: .leading)
            Text(value)
                .foregroundStyle(valueColor)
                .lineLimit(1)
                .truncationMode(.tail)
            Spacer()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
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
}

private struct KeyDeleteConfirmSheet: View {
    let slot: Int
    @Binding var input: String
    let isWorking: Bool
    let onCancel: () -> Void
    let onConfirm: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var trimmedInput: String {
        input.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var isValid: Bool {
        Int(trimmedInput) == slot
    }

    private func l(_ zh: String, _ en: String) -> String {
        ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS ? en : zh
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DesignSystem.Spacing.card) {
            Text(l("确认删除密钥", "Confirm key deletion"))
                .font(.title3.weight(.semibold))

            VStack(alignment: .leading, spacing: 8) {
                Text(l("此操作不可恢复，删除后将无法执行嵌入/检测。", "This action is irreversible. Embedding/detection will be unavailable after deletion."))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Text("\(l("请输入当前槽位号", "Enter current slot number")): \(slot)")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.primary)
            }

            GlassEffectContainer {
                TextField("\(l("输入槽位号", "Enter slot number")) \(slot)", text: $input)
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
                .disabled(!isValid || isWorking)
            }
        }
        .padding(24)
        .frame(width: 420)
    }
}

private struct EditSlotLabelSheet: View {
    let slot: Int
    let keyId: String?
    let currentLabel: String?
    @Binding var draftLabel: String
    let isWorking: Bool
    let onCancel: () -> Void
    let onConfirm: () -> Void

    private func l(_ zh: String, _ en: String) -> String {
        ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS ? en : zh
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(l("编辑槽位标签", "Edit slot label"))
                .font(.headline)
            Text("\(l("当前激活槽位", "Current active slot")): \(slot)")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text("Key ID: \(keyId ?? l("未配置", "Not configured"))")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text("\(l("当前标签", "Current label")): \(currentLabel.flatMap { $0.isEmpty ? nil : $0 } ?? l("未设置", "Not set"))")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextField(l("输入新标签（留空表示清除）", "Enter new label (leave blank to clear)"), text: $draftLabel)
                .textFieldStyle(.roundedBorder)

            HStack {
                Spacer()
                Button(l("取消", "Cancel"), action: onCancel)
                    .keyboardShortcut(.cancelAction)
                Button(l("保存", "Save"), action: onConfirm)
                    .keyboardShortcut(.defaultAction)
                    .disabled(isWorking)
            }
        }
        .padding(18)
        .frame(minWidth: 420)
    }
}

private struct HexKeyImportSheet: View {
    let slot: Int
    @Binding var draftHex: String
    let isWorking: Bool
    let onCancel: () -> Void
    let onConfirm: () -> Void

    private func l(_ zh: String, _ en: String) -> String {
        ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS ? en : zh
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(l("Hex 密钥导入", "Hex key import"))
                .font(.headline)
            Text("\(l("目标槽位", "Target slot")): \(slot)")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text(l("请输入 64 位十六进制字符（可带 0x 前缀）", "Enter 64 hex characters (0x prefix allowed)"))
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextEditor(text: $draftHex)
                .font(.system(.body, design: .monospaced))
                .frame(minHeight: 120)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.secondary.opacity(0.25), lineWidth: 1)
                )

            HStack {
                Spacer()
                Button(l("取消", "Cancel"), action: onCancel)
                    .keyboardShortcut(.cancelAction)
                Button(l("导入", "Import"), action: onConfirm)
                    .keyboardShortcut(.defaultAction)
                    .disabled(isWorking || draftHex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(18)
        .frame(minWidth: 460)
    }
}
