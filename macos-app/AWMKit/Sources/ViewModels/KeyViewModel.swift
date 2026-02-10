import Foundation
import AWMKit

@MainActor
final class KeyViewModel: ObservableObject {
    @Published var selectedSlot: Int = 0
    @Published var isWorking = false
    @Published var errorMessage: String?
    @Published var successMessage: String?
    @Published private(set) var slotSummaries: [AWMKeySlotSummary] = []

    let slotOptions: [Int] = Array(0...31)
    var selectedSlotHasKey: Bool {
        slotSummaries.first(where: { Int($0.slot) == selectedSlot })?.hasKey ?? false
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
        } catch {
            errorMessage = "生成密钥失败：\(error.localizedDescription)"
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
        } catch {
            errorMessage = "删除密钥失败：\(error.localizedDescription)"
            successMessage = nil
        }
    }

    func refreshSlotSummaries() {
        do {
            slotSummaries = try AWMKeyStore.slotSummaries()
        } catch {
            slotSummaries = []
        }
    }
}
