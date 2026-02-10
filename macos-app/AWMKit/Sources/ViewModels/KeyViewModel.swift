import Foundation
import AWMKit

@MainActor
final class KeyViewModel: ObservableObject {
    @Published var selectedSlot: Int = 0
    @Published var isWorking = false
    @Published var slotSearchText: String = ""
    @Published var isApplySuccess = false
    @Published var isGenerateSuccess = false
    @Published var isEditSuccess = false
    @Published var isDeleteSuccess = false
    @Published var isRefreshSuccess = false
    @Published var errorMessage: String?
    @Published var successMessage: String?
    @Published private(set) var slotSummaries: [AWMKeySlotSummary] = []

    let slotOptions: [Int] = Array(0...31)
    var selectedSlotHasKey: Bool {
        slotSummaries.first(where: { Int($0.slot) == selectedSlot })?.hasKey ?? false
    }
    var filteredSlotSummaries: [AWMKeySlotSummary] {
        let keyword = slotSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !keyword.isEmpty else { return slotSummaries }
        let lowered = keyword.lowercased()
        return slotSummaries.filter { summary in
            let searchable = [
                "槽位 \(summary.slot)",
                summary.keyId ?? "",
                summary.label ?? "",
                summary.statusText,
                "证据 \(summary.evidenceCount)"
            ].joined(separator: " ").lowercased()
            return searchable.contains(lowered)
        }
    }
    var activeSlotSummary: AWMKeySlotSummary? {
        slotSummaries.first(where: { $0.isActive })
    }
    var configuredSlotCount: Int {
        slotSummaries.filter(\.hasKey).count
    }

    func sync(from appState: AppState) {
        selectedSlot = appState.activeKeySlot
        refreshSlotSummaries()
    }

    func applySlot(appState: AppState) {
        appState.setActiveKeySlot(selectedSlot)
        sync(from: appState)
        successMessage = "已切换激活槽位为 \(selectedSlot)"
        errorMessage = nil
        flash(\.isApplySuccess)
    }

    func generateKey(appState: AppState) async {
        guard !isWorking else { return }
        guard !selectedSlotHasKey else {
            errorMessage = "槽位 \(selectedSlot) 已有密钥，请先删除后再生成。"
            successMessage = nil
            return
        }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.generateKey(slot: selectedSlot)
            sync(from: appState)
            successMessage = "槽位 \(selectedSlot) 密钥已生成"
            errorMessage = nil
            flash(\.isGenerateSuccess)
        } catch {
            errorMessage = "生成密钥失败：\(error.localizedDescription)"
            successMessage = nil
        }
    }

    func editActiveSlotLabel(appState: AppState, label: String) async {
        guard !isWorking else { return }
        isWorking = true
        defer { isWorking = false }

        let active = appState.activeKeySlot
        let trimmed = label.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            if trimmed.isEmpty {
                try await appState.clearSlotLabel(slot: active)
                successMessage = "槽位 \(active) 标签已清除"
            } else {
                try await appState.setSlotLabel(slot: active, label: trimmed)
                successMessage = "槽位 \(active) 标签已更新"
            }
            sync(from: appState)
            errorMessage = nil
            flash(\.isEditSuccess)
        } catch {
            errorMessage = "编辑标签失败：\(error.localizedDescription)"
            successMessage = nil
        }
    }

    func deleteKey(appState: AppState) async {
        guard !isWorking else { return }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.deleteKey(slot: selectedSlot)
            sync(from: appState)
            successMessage = "密钥已删除"
            errorMessage = nil
            flash(\.isDeleteSuccess)
        } catch {
            errorMessage = "删除密钥失败：\(error.localizedDescription)"
            successMessage = nil
        }
    }

    func refresh(appState: AppState) async {
        guard !isWorking else { return }
        isWorking = true
        defer { isWorking = false }

        await appState.refreshRuntimeStatus()
        sync(from: appState)
        flash(\.isRefreshSuccess)
    }

    func refreshSlotSummaries() {
        do {
            slotSummaries = try AWMKeyStore.slotSummaries()
        } catch {
            slotSummaries = []
        }
    }
    private func flash(_ keyPath: ReferenceWritableKeyPath<KeyViewModel, Bool>) {
        self[keyPath: keyPath] = true
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            self[keyPath: keyPath] = false
        }
    }
}
