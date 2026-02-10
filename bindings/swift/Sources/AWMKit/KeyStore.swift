import Foundation
import CAWMKit

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
}
