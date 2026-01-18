import Foundation
import Security

/// Keychain 密钥管理
public class AWMKeychain {
    /// 默认服务标识符
    public static let defaultService = "com.awmkit.watermark"

    /// 默认账户名
    public static let defaultAccount = "signing-key"

    private let service: String
    private let account: String

    /// 创建 Keychain 管理器
    ///
    /// - Parameters:
    ///   - service: 服务标识符（如 com.yourapp.watermark）
    ///   - account: 账户名（如 signing-key）
    public init(service: String = defaultService, account: String = defaultAccount) {
        self.service = service
        self.account = account
    }

    // MARK: - 基础操作

    /// 保存密钥到 Keychain
    ///
    /// - Parameter key: 密钥数据（建议 32 字节）
    /// - Throws: KeychainError
    public func saveKey(_ key: Data) throws {
        // 先尝试删除已有的
        try? deleteKey()

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecValueData as String: key,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrLabel as String: "AWMKit Watermark Signing Key",
            kSecAttrDescription as String: "HMAC key for audio watermark signing"
        ]

        let status = SecItemAdd(query as CFDictionary, nil)

        if status != errSecSuccess {
            throw KeychainError.saveFailed(status)
        }
    }

    /// 从 Keychain 读取密钥
    ///
    /// - Returns: 密钥数据，不存在返回 nil
    /// - Throws: KeychainError
    public func loadKey() throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        switch status {
        case errSecSuccess:
            return result as? Data
        case errSecItemNotFound:
            return nil
        default:
            throw KeychainError.loadFailed(status)
        }
    }

    /// 删除 Keychain 中的密钥
    ///
    /// - Throws: KeychainError
    public func deleteKey() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        let status = SecItemDelete(query as CFDictionary)

        if status != errSecSuccess && status != errSecItemNotFound {
            throw KeychainError.deleteFailed(status)
        }
    }

    /// 检查密钥是否存在
    public var hasKey: Bool {
        (try? loadKey()) != nil
    }

    // MARK: - 便捷方法

    /// 从文件导入密钥
    ///
    /// - Parameter url: 密钥文件路径
    /// - Throws: KeychainError
    public func importKey(from url: URL) throws {
        let key = try Data(contentsOf: url)
        try saveKey(key)
    }

    /// 导出密钥到文件
    ///
    /// - Parameter url: 目标文件路径
    /// - Throws: KeychainError
    public func exportKey(to url: URL) throws {
        guard let key = try loadKey() else {
            throw KeychainError.keyNotFound
        }
        try key.write(to: url)
    }

    /// 生成随机密钥并保存
    ///
    /// - Parameter length: 密钥长度（默认 32 字节）
    /// - Returns: 生成的密钥
    /// - Throws: KeychainError
    @discardableResult
    public func generateAndSaveKey(length: Int = 32) throws -> Data {
        var bytes = [UInt8](repeating: 0, count: length)
        let status = SecRandomCopyBytes(kSecRandomDefault, length, &bytes)

        if status != errSecSuccess {
            throw KeychainError.generateFailed(status)
        }

        let key = Data(bytes)
        try saveKey(key)
        return key
    }
}

// MARK: - 错误类型

public enum KeychainError: Error, LocalizedError {
    case saveFailed(OSStatus)
    case loadFailed(OSStatus)
    case deleteFailed(OSStatus)
    case generateFailed(OSStatus)
    case keyNotFound

    public var errorDescription: String? {
        switch self {
        case .saveFailed(let status):
            return "Failed to save key to Keychain: \(status)"
        case .loadFailed(let status):
            return "Failed to load key from Keychain: \(status)"
        case .deleteFailed(let status):
            return "Failed to delete key from Keychain: \(status)"
        case .generateFailed(let status):
            return "Failed to generate random key: \(status)"
        case .keyNotFound:
            return "Key not found in Keychain"
        }
    }
}

// MARK: - 全局便捷访问

extension AWMKeychain {
    /// 共享实例（使用默认服务和账户）
    public static let shared = AWMKeychain()

    /// 快速保存密钥
    public static func save(_ key: Data) throws {
        try shared.saveKey(key)
    }

    /// 快速读取密钥
    public static func load() throws -> Data? {
        try shared.loadKey()
    }

    /// 快速读取密钥（不存在则抛出错误）
    public static func require() throws -> Data {
        guard let key = try shared.loadKey() else {
            throw KeychainError.keyNotFound
        }
        return key
    }
}
