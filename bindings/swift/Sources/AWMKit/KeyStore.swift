import Foundation
import CAWMKit

public struct AWMKeySlotSummary: Codable, Hashable {
    public let slot: UInt8
    public let isActive: Bool
    public let hasKey: Bool
    public let keyId: String?
    public let label: String?
    public let evidenceCount: Int
    public let lastEvidenceAt: UInt64?
    public let statusText: String
    public let duplicateOfSlots: [UInt8]

    enum CodingKeys: String, CodingKey {
        case slot
        case isActive = "is_active"
        case hasKey = "has_key"
        case keyId = "key_id"
        case label
        case evidenceCount = "evidence_count"
        case lastEvidenceAt = "last_evidence_at"
        case statusText = "status_text"
        case duplicateOfSlots = "duplicate_of_slots"
    }
}

/// Slot-aware key operations bridged from Rust KeyStore.
public enum AWMKeyStore {
    private static let keyLength = 32
    private static let labelBufferSize = 512

    public static func exists() -> Bool {
        awm_key_exists()
    }

    public static func exists(slot: UInt8) -> Bool {
        awm_key_exists_slot(slot)
    }

    public static func backendLabel() throws -> String {
        var buffer = [CChar](repeating: 0, count: labelBufferSize)
        let code = awm_key_backend_label(&buffer, buffer.count)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return String(cString: buffer)
    }

    public static func activeSlot() throws -> UInt8 {
        var slot: UInt8 = 0
        let code = awm_key_active_slot_get(&slot)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return slot
    }

    public static func setActiveSlot(_ slot: UInt8) throws {
        let code = awm_key_active_slot_set(slot)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
    }

    public static func slotSummaries() throws -> [AWMKeySlotSummary] {
        let json = try fetchCString { out, outLen, required in
            awm_key_slot_summaries_json(out, outLen, required)
        }
        let payload = json.isEmpty ? "[]" : json
        let data = Data(payload.utf8)
        return try JSONDecoder().decode([AWMKeySlotSummary].self, from: data)
    }

    public static func loadActiveKey() throws -> Data {
        var key = [UInt8](repeating: 0, count: keyLength)
        let code = key.withUnsafeMutableBufferPointer { buffer -> Int32 in
            awm_key_load(buffer.baseAddress, keyLength)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return Data(key)
    }

    @discardableResult
    public static func generateAndSaveActiveKey() throws -> Data {
        var key = [UInt8](repeating: 0, count: keyLength)
        let code = key.withUnsafeMutableBufferPointer { buffer -> Int32 in
            awm_key_generate_and_save(buffer.baseAddress, keyLength)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return Data(key)
    }

    @discardableResult
    public static func generateAndSaveKey(slot: UInt8) throws -> Data {
        var key = [UInt8](repeating: 0, count: keyLength)
        let code = key.withUnsafeMutableBufferPointer { buffer -> Int32 in
            awm_key_generate_and_save_slot(slot, buffer.baseAddress, keyLength)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return Data(key)
    }

    /// Delete key in target slot and return effective active slot after fallback.
    @discardableResult
    public static func deleteKey(slot: UInt8) throws -> UInt8 {
        var nextActive: UInt8 = 0
        let code = awm_key_delete_slot(slot, &nextActive)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return nextActive
    }

    /// Delete key in current active slot and return effective active slot after fallback.
    @discardableResult
    public static func deleteActiveKey() throws -> UInt8 {
        let slot = try activeSlot()
        return try deleteKey(slot: slot)
    }

    private static func fetchCString(
        _ caller: (_ out: UnsafeMutablePointer<CChar>?, _ outLen: Int, _ required: UnsafeMutablePointer<Int>?) -> Int32
    ) throws -> String {
        var requiredLen = 0
        var code = caller(nil, 0, &requiredLen)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }

        let size = max(requiredLen, 1)
        var buffer = [CChar](repeating: 0, count: size)
        code = caller(&buffer, buffer.count, &requiredLen)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return String(cString: buffer)
    }
}
