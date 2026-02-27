import AppKit
import Foundation
import AWMKit
import UniformTypeIdentifiers

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
    @Published var isImportSuccess = false
    @Published var isHexImportSuccess = false
    @Published var isExportSuccess = false
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
                "\(Localizer.pick("槽位", "Slot")) \(summary.slot)",
                summary.keyId ?? "",
                summary.label ?? "",
                summary.statusText,
                "\(Localizer.pick("证据", "Evidence")) \(summary.evidenceCount)"
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
        successMessage = "\(Localizer.pick("已切换激活槽位为", "Switched active slot to")) \(selectedSlot)"
        errorMessage = nil
        flash(\.isApplySuccess)
    }

    func generateKey(appState: AppState) async {
        guard !isWorking else { return }
        guard !selectedSlotHasKey else {
            errorMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 已有密钥，请先删除后再生成。", " already has a key. Delete it before generating a new one."))"
            successMessage = nil
            return
        }
        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.generateKey(slot: selectedSlot)
            sync(from: appState)
            successMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 密钥已生成", " key generated"))"
            errorMessage = nil
            flash(\.isGenerateSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("生成密钥失败", "Failed to generate key")): \(error.localizedDescription)"
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
                successMessage = "\(Localizer.pick("槽位", "Slot")) \(active)\(Localizer.pick(" 标签已清除", " label cleared"))"
            } else {
                try await appState.setSlotLabel(slot: active, label: trimmed)
                successMessage = "\(Localizer.pick("槽位", "Slot")) \(active)\(Localizer.pick(" 标签已更新", " label updated"))"
            }
            sync(from: appState)
            errorMessage = nil
            flash(\.isEditSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("编辑标签失败", "Failed to edit label")): \(error.localizedDescription)"
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
            successMessage = Localizer.pick("密钥已删除", "Key deleted")
            errorMessage = nil
            flash(\.isDeleteSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("删除密钥失败", "Failed to delete key")): \(error.localizedDescription)"
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

    func importKeyFromFile(appState: AppState) async {
        guard !isWorking else { return }
        guard !selectedSlotHasKey else {
            errorMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 已有密钥，请先删除后再导入。", " already has a key. Delete it before importing."))"
            successMessage = nil
            return
        }

        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        panel.allowedContentTypes = [UTType.data]
        panel.message = Localizer.pick("选择 32 字节密钥文件（.bin）", "Select a 32-byte key file (.bin)")

        guard panel.runModal() == .OK, let url = panel.url else {
            return
        }

        isWorking = true
        defer { isWorking = false }

        do {
            let key = try Data(contentsOf: url)
            guard key.count == 32 else {
                errorMessage = "\(Localizer.pick("导入密钥失败", "Import failed")): \(Localizer.pick("文件长度必须为 32 字节", "Key file must be exactly 32 bytes")) (\(key.count))"
                successMessage = nil
                return
            }

            try await appState.saveKey(slot: selectedSlot, key: key)
            refreshSlotSummaries()
            successMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 密钥导入成功", " key imported"))"
            errorMessage = nil
            flash(\.isImportSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("导入密钥失败", "Import failed")): \(error.localizedDescription)"
            successMessage = nil
        }
    }

    func importKeyFromHex(appState: AppState, hexInput: String) async {
        guard !isWorking else { return }
        guard !selectedSlotHasKey else {
            errorMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 已有密钥，请先删除后再导入。", " already has a key. Delete it before importing."))"
            successMessage = nil
            return
        }

        guard let normalized = normalizedHexKey(hexInput),
              let key = Data(hexString: normalized),
              key.count == 32 else {
            errorMessage = "\(Localizer.pick("Hex 导入失败", "Hex import failed")): \(Localizer.pick("请输入 64 位十六进制字符（可带 0x 前缀）", "Enter 64 hex characters (0x prefix allowed)"))"
            successMessage = nil
            return
        }

        isWorking = true
        defer { isWorking = false }

        do {
            try await appState.saveKey(slot: selectedSlot, key: key)
            refreshSlotSummaries()
            successMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" Hex 密钥导入成功", " hex key imported"))"
            errorMessage = nil
            flash(\.isHexImportSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("Hex 导入失败", "Hex import failed")): \(error.localizedDescription)"
            successMessage = nil
        }
    }

    func exportKeyToFile(appState: AppState) async {
        guard !isWorking else { return }
        guard selectedSlotHasKey else {
            errorMessage = Localizer.pick("当前槽位无密钥可导出", "Selected slot has no key to export")
            successMessage = nil
            return
        }

        isWorking = true
        defer { isWorking = false }

        do {
            let key = try appState.loadKey(slot: selectedSlot)
            let panel = NSSavePanel()
            panel.canCreateDirectories = true
            panel.allowedContentTypes = [UTType.data]
            panel.nameFieldStringValue = "awmkit-key-slot-\(selectedSlot).bin"
            panel.message = Localizer.pick("导出 32 字节密钥文件（.bin）", "Export 32-byte key file (.bin)")

            guard panel.runModal() == .OK, let url = panel.url else {
                return
            }

            try key.write(to: url, options: .atomic)
            successMessage = "\(Localizer.pick("槽位", "Slot")) \(selectedSlot)\(Localizer.pick(" 密钥导出成功", " key exported"))"
            errorMessage = nil
            flash(\.isExportSuccess)
        } catch {
            errorMessage = "\(Localizer.pick("导出密钥失败", "Export failed")): \(error.localizedDescription)"
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
    private func flash(_ keyPath: ReferenceWritableKeyPath<KeyViewModel, Bool>) {
        self[keyPath: keyPath] = true
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            self[keyPath: keyPath] = false
        }
    }

    private func normalizedHexKey(_ input: String) -> String? {
        let compact = input.replacingOccurrences(of: "\\s+", with: "", options: .regularExpression)
        guard !compact.isEmpty else { return nil }

        let normalized: String
        if compact.lowercased().hasPrefix("0x") {
            normalized = String(compact.dropFirst(2))
        } else {
            normalized = compact
        }

        guard normalized.count == 64 else { return nil }
        let isHex = normalized.unicodeScalars.allSatisfy { scalar in
            CharacterSet(charactersIn: "0123456789abcdefABCDEF").contains(scalar)
        }
        return isHex ? normalized : nil
    }
}
