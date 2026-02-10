import SwiftUI
import AWMKit

struct KeyView: View {
    @EnvironmentObject private var appState: AppState
    @ObservedObject var viewModel: KeyViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var showDeleteConfirm = false

    var body: some View {
        GeometryReader { proxy in
            HStack(alignment: .top, spacing: DesignSystem.Spacing.card) {
                GlassCard {
                    VStack(alignment: .leading, spacing: 14) {
                        header("密钥管理")
                        statusSection
                        slotSection
                        activeKeyCapsule
                        actionSection
                        hintSection
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
                .frame(width: max(420, proxy.size.width * 0.42))

                GlassCard {
                    VStack(alignment: .leading, spacing: 12) {
                        header("槽位摘要", count: viewModel.configuredSlotCount)
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
        .onChange(of: viewModel.selectedSlot) { _ in
            viewModel.syncLabelInputForSelectedSlot()
        }
        .alert("删除密钥", isPresented: $showDeleteConfirm) {
            Button("取消", role: .cancel) {}
            Button("删除", role: .destructive) {
                Task { await viewModel.deleteKey(appState: appState) }
            }
        } message: {
            Text("删除后将无法执行嵌入/检测，是否继续？")
        }
        .alert("操作结果", isPresented: Binding(
            get: { viewModel.errorMessage != nil || viewModel.successMessage != nil },
            set: { newValue in
                if !newValue {
                    viewModel.errorMessage = nil
                    viewModel.successMessage = nil
                }
            })
        ) {
            Button("确定", role: .cancel) {}
        } message: {
            Text(viewModel.errorMessage ?? viewModel.successMessage ?? "")
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
                label: "密钥状态",
                value: appState.keyLoaded ? "已配置" : "未配置",
                valueColor: appState.keyLoaded ? DesignSystem.Colors.success : DesignSystem.Colors.warning
            )
            row(label: "密钥来源", value: appState.keySourceLabel)
        }
    }

    private var slotSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("激活槽位")
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
                        Text("应用")
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
            }

            Text("当前激活槽位：\(appState.activeKeySlot)")
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
            HStack(spacing: 8) {
                Image(systemName: "tag")
                    .foregroundStyle(.secondary)
                TextField("标签（可选，生成时一并写入）", text: $viewModel.labelInput)
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

            HStack(spacing: 10) {
                Button {
                    Task { await viewModel.generateKey(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "key.fill")
                            .foregroundStyle(viewModel.isGenerateSuccess ? DesignSystem.Colors.success : .primary)
                        Text("生成")
                    }
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(viewModel.isWorking)

                Button {
                    Task { await viewModel.editActiveSlotLabel(appState: appState) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "tag.fill")
                            .foregroundStyle(viewModel.isEditSuccess ? DesignSystem.Colors.success : .primary)
                        Text("编辑")
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.isWorking)

                Button {
                    showDeleteConfirm = true
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "trash")
                            .foregroundStyle(viewModel.isDeleteSuccess ? DesignSystem.Colors.success : .primary)
                        Text("删除")
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
                        Text("刷新")
                    }
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
                .disabled(viewModel.isWorking)
            }
        }
    }

    private var hintSection: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("槽位只能在密钥页修改。")
            Text("当前版本嵌入仍写槽位 0，槽位切换将在后续协议生效阶段接入。")
        }
        .font(.caption)
        .foregroundStyle(.secondary)
    }

    private var slotSearchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
            TextField("搜索槽位 / Key ID / 标签 / 状态", text: $viewModel.slotSearchText)
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
        let keyText = summary?.hasKey == true ? "Key ID: \(summary?.keyId ?? "-")" : "未配置"
        let labelText = (summary?.label?.isEmpty == false) ? " · \(summary?.label ?? "")" : ""
        let evidenceText = "证据: \(summary?.evidenceCount ?? 0)"
        return VStack(alignment: .leading, spacing: 6) {
            Text("当前激活密钥")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Image(systemName: "square.grid.2x2")
                        .foregroundStyle(summary?.isActive == true ? DesignSystem.Colors.success : .secondary)
                    Text("槽位 \(appState.activeKeySlot)\(summary?.hasKey == true ? "（已配置）" : "（未配置）")")
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
        let title = summary.isActive ? "槽位 \(summary.slot)（激活）" : "槽位 \(summary.slot)"
        let keyText = summary.hasKey ? "Key ID: \(summary.keyId ?? "-")" : "未配置"
        let labelText = summary.label?.isEmpty == false ? " · \(summary.label!)" : ""
        let evidenceText = "证据: \(summary.evidenceCount)"
        let duplicateText = summary.duplicateOfSlots.isEmpty
            ? ""
            : " · 重复: \(summary.duplicateOfSlots.map(String.init).joined(separator: ","))"

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
