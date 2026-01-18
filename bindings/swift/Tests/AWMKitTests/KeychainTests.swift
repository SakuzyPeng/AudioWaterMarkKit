import XCTest
@testable import AWMKit

final class KeychainTests: XCTestCase {
    // 使用测试专用的 service，避免影响真实数据
    let testKeychain = AWMKeychain(
        service: "com.awmkit.test",
        account: "test-key"
    )

    override func tearDown() {
        // 清理测试数据
        try? testKeychain.deleteKey()
    }

    func testSaveAndLoad() throws {
        let testKey = Data("test-key-32-bytes-for-hmac-test!".utf8)

        // 保存
        try testKeychain.saveKey(testKey)

        // 读取
        let loaded = try testKeychain.loadKey()
        XCTAssertEqual(loaded, testKey)
    }

    func testHasKey() throws {
        XCTAssertFalse(testKeychain.hasKey)

        try testKeychain.saveKey(Data("test".utf8))
        XCTAssertTrue(testKeychain.hasKey)

        try testKeychain.deleteKey()
        XCTAssertFalse(testKeychain.hasKey)
    }

    func testDelete() throws {
        try testKeychain.saveKey(Data("test".utf8))
        XCTAssertTrue(testKeychain.hasKey)

        try testKeychain.deleteKey()
        XCTAssertFalse(testKeychain.hasKey)
    }

    func testGenerateKey() throws {
        let key = try testKeychain.generateAndSaveKey()

        XCTAssertEqual(key.count, 32)  // 默认 32 字节
        XCTAssertTrue(testKeychain.hasKey)

        let loaded = try testKeychain.loadKey()
        XCTAssertEqual(loaded, key)
    }

    func testGenerateKeyCustomLength() throws {
        let key = try testKeychain.generateAndSaveKey(length: 64)
        XCTAssertEqual(key.count, 64)
    }

    func testOverwrite() throws {
        let key1 = Data("first-key".utf8)
        let key2 = Data("second-key".utf8)

        try testKeychain.saveKey(key1)
        try testKeychain.saveKey(key2)  // 应该覆盖

        let loaded = try testKeychain.loadKey()
        XCTAssertEqual(loaded, key2)
    }

    func testLoadNonexistent() throws {
        let loaded = try testKeychain.loadKey()
        XCTAssertNil(loaded)
    }

    func testImportExport() throws {
        let tempDir = FileManager.default.temporaryDirectory
        let keyFile = tempDir.appendingPathComponent("test-key.bin")

        // 创建测试密钥文件
        let testKey = Data("import-export-test-key-32-bytes!".utf8)
        try testKey.write(to: keyFile)

        defer {
            try? FileManager.default.removeItem(at: keyFile)
        }

        // 导入
        try testKeychain.importKey(from: keyFile)
        let loaded = try testKeychain.loadKey()
        XCTAssertEqual(loaded, testKey)

        // 导出
        let exportFile = tempDir.appendingPathComponent("exported-key.bin")
        defer {
            try? FileManager.default.removeItem(at: exportFile)
        }

        try testKeychain.exportKey(to: exportFile)
        let exported = try Data(contentsOf: exportFile)
        XCTAssertEqual(exported, testKey)
    }

    func testExportNonexistent() {
        let tempFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("nonexistent.bin")

        XCTAssertThrowsError(try testKeychain.exportKey(to: tempFile)) { error in
            guard case KeychainError.keyNotFound = error else {
                XCTFail("Expected keyNotFound error")
                return
            }
        }
    }

    func testDifferentAccounts() throws {
        let keychain1 = AWMKeychain(service: "com.awmkit.test", account: "account1")
        let keychain2 = AWMKeychain(service: "com.awmkit.test", account: "account2")

        defer {
            try? keychain1.deleteKey()
            try? keychain2.deleteKey()
        }

        let key1 = Data("key-for-account-1".utf8)
        let key2 = Data("key-for-account-2".utf8)

        try keychain1.saveKey(key1)
        try keychain2.saveKey(key2)

        XCTAssertEqual(try keychain1.loadKey(), key1)
        XCTAssertEqual(try keychain2.loadKey(), key2)
    }
}
