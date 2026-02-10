import SwiftUI

struct KeyView: View {
    @EnvironmentObject private var appState: AppState
    @ObservedObject var viewModel: KeyViewModel
    @Environment(\.colorScheme) private var colorScheme
    @State private var showDeleteConfirm = false

    var body: some View {
        GeometryReader { proxy in
            VStack(alignment: .leading, spacing: DesignSystem.Spacing.card) {
                GlassCard {
                    VStack(alignment: .leading, spacing: 14) {
                        header
                        statusSection
                        slotSection
                        actionSection
                        hintSection
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

    private var header: some View {
        HStack {
            Text("密钥管理")
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
    }

    private var statusSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            row(label: "密钥状态", value: appState.keyLoaded ? "已配置" : "未配置")
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
                Button("应用槽位") {
                    viewModel.applySlot(appState: appState)
                }
                .buttonStyle(GlassButtonStyle(size: .compact))
            }

            Text("当前激活槽位：\(appState.activeKeySlot)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private var actionSection: some View {
        HStack(spacing: 10) {
            Button {
                Task { await viewModel.generateKey(appState: appState) }
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "plus")
                    Text("生成密钥")
                }
            }
            .buttonStyle(GlassButtonStyle(accentOn: true))
            .disabled(viewModel.isWorking)

            Button {
                showDeleteConfirm = true
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "trash")
                    Text("删除密钥")
                }
            }
            .buttonStyle(GlassButtonStyle(size: .compact))
            .disabled(!viewModel.selectedSlotHasKey || viewModel.isWorking)

            Button {
                Task {
                    await appState.refreshRuntimeStatus()
                    viewModel.sync(from: appState)
                }
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "arrow.clockwise")
                    Text("刷新状态")
                }
            }
            .buttonStyle(GlassButtonStyle(size: .compact))
            .disabled(viewModel.isWorking)
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

    private func row(label: String, value: String) -> some View {
        HStack(spacing: 12) {
            Text(label)
                .foregroundStyle(.secondary)
                .frame(width: 70, alignment: .leading)
            Text(value)
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
