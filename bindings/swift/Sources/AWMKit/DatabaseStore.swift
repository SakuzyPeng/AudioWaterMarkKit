import Foundation
import CAWMKit

public struct AWMDatabaseTagEntry: Codable, Hashable {
    public let username: String
    public let tag: String
    public let createdAt: UInt64

    enum CodingKeys: String, CodingKey {
        case username
        case tag
        case createdAt = "created_at"
    }
}

public struct AWMDatabaseEvidenceEntry: Codable, Hashable {
    public let id: Int64
    public let createdAt: UInt64
    public let filePath: String
    public let tag: String
    public let identity: String
    public let version: UInt8
    public let keySlot: UInt8
    public let timestampMinutes: UInt32
    public let messageHex: String
    public let sampleRate: UInt32
    public let channels: UInt32
    public let sampleCount: UInt64
    public let pcmSha256: String
    public let keyId: String?
    public let snrDb: Double?
    public let snrStatus: String
    public let chromaprintBlob: String
    public let fingerprintLen: Int
    public let fpConfigId: UInt8

    enum CodingKeys: String, CodingKey {
        case id
        case createdAt = "created_at"
        case filePath = "file_path"
        case tag
        case identity
        case version
        case keySlot = "key_slot"
        case timestampMinutes = "timestamp_minutes"
        case messageHex = "message_hex"
        case sampleRate = "sample_rate"
        case channels
        case sampleCount = "sample_count"
        case pcmSha256 = "pcm_sha256"
        case keyId = "key_id"
        case snrDb = "snr_db"
        case snrStatus = "snr_status"
        case chromaprintBlob = "chromaprint_blob"
        case fingerprintLen = "fingerprint_len"
        case fpConfigId = "fp_config_id"
    }
}

public enum AWMDatabaseStore {
    public static func summary() throws -> (tagCount: Int, evidenceCount: Int) {
        var tagCount: UInt64 = 0
        var evidenceCount: UInt64 = 0
        let code = awm_db_summary(&tagCount, &evidenceCount)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return (Int(tagCount), Int(evidenceCount))
    }

    public static func listTagMappings(limit: Int = 200) throws -> [AWMDatabaseTagEntry] {
        let normalized = UInt32(max(1, min(limit, Int(UInt32.max))))
        let json = try fetchJSONString { out, outLen, required in
            awm_db_tag_list_json(normalized, out, outLen, required)
        }
        let data = Data(json.utf8)
        return try JSONDecoder().decode([AWMDatabaseTagEntry].self, from: data)
    }

    public static func lookupTag(username: String) throws -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }

        let value = try fetchCString { out, outLen, required in
            normalized.withCString { usernamePtr in
                awm_db_tag_lookup(usernamePtr, out, outLen, required)
            }
        }

        return value.isEmpty ? nil : value
    }

    public static func saveTagIfAbsent(username: String, tag: String) throws -> Bool {
        let normalizedUser = username.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalizedTag = tag.uppercased()
        guard !normalizedUser.isEmpty else {
            throw AWMError.invalidTag("username is empty")
        }

        var inserted = false
        let code = normalizedUser.withCString { userPtr in
            normalizedTag.withCString { tagPtr in
                awm_db_tag_save_if_absent(userPtr, tagPtr, &inserted)
            }
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return inserted
    }

    public static func removeTagMappings(usernames: [String]) throws -> Int {
        let normalized = usernames
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        let payload = try JSONEncoder().encode(normalized)
        guard let json = String(data: payload, encoding: .utf8) else {
            throw AWMError.invalidUtf8
        }

        var deleted: UInt32 = 0
        let code = json.withCString { jsonPtr in
            awm_db_tag_remove_json(jsonPtr, &deleted)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return Int(deleted)
    }

    public static func listEvidence(limit: Int = 200) throws -> [AWMDatabaseEvidenceEntry] {
        let normalized = UInt32(max(1, min(limit, Int(UInt32.max))))
        let json = try fetchJSONString { out, outLen, required in
            awm_db_evidence_list_json(normalized, out, outLen, required)
        }
        let data = Data(json.utf8)
        return try JSONDecoder().decode([AWMDatabaseEvidenceEntry].self, from: data)
    }

    public static func removeEvidence(ids: [Int64]) throws -> Int {
        let payload = try JSONEncoder().encode(Array(Set(ids)))
        guard let json = String(data: payload, encoding: .utf8) else {
            throw AWMError.invalidUtf8
        }

        var deleted: UInt32 = 0
        let code = json.withCString { jsonPtr in
            awm_db_evidence_remove_json(jsonPtr, &deleted)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return Int(deleted)
    }

    public static func suggestTag(username: String) throws -> String {
        var buffer = [CChar](repeating: 0, count: 9)
        let code = username.withCString { ptr in
            awm_tag_suggest(ptr, &buffer)
        }
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return String(cString: buffer)
    }

    private static func fetchJSONString(
        _ caller: (_ out: UnsafeMutablePointer<CChar>?, _ outLen: Int, _ required: UnsafeMutablePointer<Int>?) -> Int32
    ) throws -> String {
        let value = try fetchCString(caller)
        return value.isEmpty ? "[]" : value
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
