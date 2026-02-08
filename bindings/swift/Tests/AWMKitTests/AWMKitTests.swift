import XCTest
@testable import AWMKit

final class AWMKitTests: XCTestCase {
    let testKey = Data("test-key-32-bytes-for-hmac-test!".utf8)

    func testTagCreation() throws {
        let tag = try AWMTag(identity: "SAKUZY")
        XCTAssertEqual(tag.identity, "SAKUZY")
        XCTAssertEqual(tag.value.count, 8)
        XCTAssertTrue(tag.isValid)
    }

    func testTagParsing() throws {
        let tag1 = try AWMTag(identity: "SAKUZY")
        let tag2 = try AWMTag(tag: tag1.value)
        XCTAssertEqual(tag1, tag2)
    }

    func testTagCaseInsensitive() throws {
        let tag1 = try AWMTag(identity: "sakuzy")
        let tag2 = try AWMTag(identity: "SAKUZY")
        XCTAssertEqual(tag1.value, tag2.value)
    }

    func testInvalidTagChecksum() {
        XCTAssertThrowsError(try AWMTag(tag: "SAKUZY_A")) { error in
            guard case AWMError.checksumMismatch = error else {
                XCTFail("Expected checksumMismatch error")
                return
            }
        }
    }

    func testMessageEncodeDecode() throws {
        let tag = try AWMTag(identity: "SAKUZY")
        let msg = try AWMMessage.encode(tag: tag, key: testKey)

        XCTAssertEqual(msg.count, 16)

        let result = try AWMMessage.decode(msg, key: testKey)
        XCTAssertEqual(result.version, AWMMessage.currentVersion)
        XCTAssertEqual(result.identity, "SAKUZY")
        XCTAssertEqual(result.tag, tag)
        XCTAssertEqual(result.keySlot, 0)
    }

    func testMessageWithTimestamp() throws {
        let tag = try AWMTag(identity: "TEST")
        let tsMinutes: UInt32 = 29049600 // 2026-01-18 00:00 UTC

        let msg = try AWMMessage.encode(
            tag: tag,
            key: testKey,
            timestampMinutes: tsMinutes
        )

        let result = try AWMMessage.decode(msg, key: testKey)
        XCTAssertEqual(result.timestampMinutes, tsMinutes)
        XCTAssertEqual(result.timestampUTC, UInt64(tsMinutes) * 60)
        XCTAssertEqual(result.keySlot, 0)
    }

    func testMessageWrongKey() throws {
        let tag = try AWMTag(identity: "SAKUZY")
        let msg = try AWMMessage.encode(tag: tag, key: testKey)

        let wrongKey = Data("wrong-key-32-bytes-for-hmac!!!!".utf8)
        XCTAssertThrowsError(try AWMMessage.decode(msg, key: wrongKey)) { error in
            guard case AWMError.hmacMismatch = error else {
                XCTFail("Expected hmacMismatch error")
                return
            }
        }
    }

    func testMessageVerify() throws {
        let tag = try AWMTag(identity: "SAKUZY")
        let msg = try AWMMessage.encode(tag: tag, key: testKey)

        XCTAssertTrue(AWMMessage.verify(msg, key: testKey))

        let wrongKey = Data("wrong-key".utf8)
        XCTAssertFalse(AWMMessage.verify(msg, key: wrongKey))
    }

    func testMessageTampered() throws {
        let tag = try AWMTag(identity: "SAKUZY")
        var msg = try AWMMessage.encode(tag: tag, key: testKey)

        // Tamper with timestamp
        msg[2] ^= 0x01

        XCTAssertFalse(AWMMessage.verify(msg, key: testKey))
        XCTAssertThrowsError(try AWMMessage.decode(msg, key: testKey))
    }

    func testHexEncoding() {
        let data = Data([0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF])
        XCTAssertEqual(data.hexString, "0123456789abcdef")

        let decoded = Data(hexString: "0123456789abcdef")
        XCTAssertEqual(decoded, data)
    }

    func testTagCodable() throws {
        let tag = try AWMTag(identity: "SAKUZY")

        let encoder = JSONEncoder()
        let data = try encoder.encode(tag)

        let decoder = JSONDecoder()
        let decoded = try decoder.decode(AWMTag.self, from: data)

        XCTAssertEqual(tag, decoded)
    }
}
