import Foundation
import AWMKit

@MainActor
final class KeyViewModel: ObservableObject {
    @Published var selectedSlot: Int = 0
    @Published var isWorking = false
    @Published var errorMessage: String?
    @Published var successMessage: String?

    let slotOptions: [Int] = Array(0...31)
    var selectedSlotHasKey: Bool {
        let normalized = max(0, min(31, selectedSlot))
        return AWMKeyStore.exists(slot: UInt8(normalized))
    }

    func sync(from appState: AppState) {
        selectedSlot = appState.activeKeySlot
    }

    func applySlot(appState: AppState) {
        appState.setActiveKeySlot(selectedSlot)
        sync(from: appState)
        successMessage = "已切换激活槽位为 \(selectedSlot)"
        errorMessage = nil
    }

    func generateKey(appState: AppState) async {
        guard !isWorking else { return }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.generateKey(slot: selectedSlot)
            sync(from: appState)
            successMessage = "密钥已生成"
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
}
