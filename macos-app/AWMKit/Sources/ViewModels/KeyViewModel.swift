import Foundation

@MainActor
final class KeyViewModel: ObservableObject {
    @Published var selectedSlot: Int = 0
    @Published var isWorking = false
    @Published var errorMessage: String?
    @Published var successMessage: String?

    let slotOptions: [Int] = Array(0...31)

    func sync(from appState: AppState) {
        selectedSlot = appState.activeKeySlot
    }

    func applySlot(appState: AppState) {
        appState.setActiveKeySlot(selectedSlot)
        sync(from: appState)
        successMessage = "已切换激活槽位为 \(selectedSlot)"
    }

    func generateKey(appState: AppState) async {
        guard !isWorking else { return }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.generateKey()
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
            try await appState.deleteKey()
            successMessage = "密钥已删除"
            errorMessage = nil
        } catch {
            errorMessage = "删除密钥失败：\(error.localizedDescription)"
            successMessage = nil
        }
    }
}
