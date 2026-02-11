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
                "\(localized("槽位", "Slot")) \(summary.slot)",
                summary.keyId ?? "",
                summary.label ?? "",
                summary.statusText,
                "\(localized("证据", "Evidence")) \(summary.evidenceCount)"
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
        successMessage = "\(localized("已切换激活槽位为", "Switched active slot to")) \(selectedSlot)"
        errorMessage = nil
        flash(\.isApplySuccess)
    }

    func generateKey(appState: AppState) async {
        guard !isWorking else { return }
        guard !selectedSlotHasKey else {
            errorMessage = "\(localized("槽位", "Slot")) \(selectedSlot)\(localized(" 已有密钥，请先删除后再生成。", " already has a key. Delete it before generating a new one."))"
            successMessage = nil
            return
        }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.generateKey(slot: selectedSlot)
            sync(from: appState)
            successMessage = "\(localized("槽位", "Slot")) \(selectedSlot)\(localized(" 密钥已生成", " key generated"))"
            errorMessage = nil
            flash(\.isGenerateSuccess)
        } catch {
            errorMessage = "\(localized("生成密钥失败", "Failed to generate key")): \(error.localizedDescription)"
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
                successMessage = "\(localized("槽位", "Slot")) \(active)\(localized(" 标签已清除", " label cleared"))"
            } else {
                try await appState.setSlotLabel(slot: active, label: trimmed)
                successMessage = "\(localized("槽位", "Slot")) \(active)\(localized(" 标签已更新", " label updated"))"
            }
            sync(from: appState)
            errorMessage = nil
            flash(\.isEditSuccess)
        } catch {
            errorMessage = "\(localized("编辑标签失败", "Failed to edit label")): \(error.localizedDescription)"
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
            successMessage = localized("密钥已删除", "Key deleted")
            errorMessage = nil
            flash(\.isDeleteSuccess)
        } catch {
            errorMessage = "\(localized("删除密钥失败", "Failed to delete key")): \(error.localizedDescription)"
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

    private func localized(_ zh: String, _ en: String) -> String {
        ((try? AWMUILanguageStore.get()) ?? .zhCN) == .enUS ? en : zh
    }
}
