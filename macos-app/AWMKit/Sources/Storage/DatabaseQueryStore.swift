import Foundation
import AWMKit

struct TagMappingEntry: Equatable, Hashable {
    let username: String
    let tag: String
    let createdAt: Int64
}

struct EvidenceEntry: Identifiable, Equatable {
    let id: Int64
    let createdAt: Int64
    let filePath: String
    let tag: String
    let identity: String
    let version: Int
    let keySlot: Int
    let timestampMinutes: Int64
    let pcmSha256: String
    let keyId: String?
    let isForcedEmbed: Bool
    let snrDb: Double?
    let snrStatus: String

    var createdDate: Date {
        Date(timeIntervalSince1970: TimeInterval(createdAt))
    }
}

enum DatabaseQueryStoreError: LocalizedError {
    case emptyUsername
    case mappingNotFound(String)

    var errorDescription: String? {
        switch self {
        case .emptyUsername:
            return "用户名不能为空"
        case .mappingNotFound(let username):
            return "未找到用户映射: \(username)"
        }
    }
}

enum DatabaseQueryStore {
    static func listTagMappings(limit: Int = 200) throws -> [TagMappingEntry] {
        try AWMDatabaseStore.listTagMappings(limit: max(1, limit)).map { row in
            TagMappingEntry(
                username: row.username.trimmingCharacters(in: .whitespacesAndNewlines),
                tag: row.tag.uppercased(),
                createdAt: toInt64(row.createdAt)
            )
        }
    }

    static func saveTagMapping(username: String) throws -> [TagMappingEntry] {
        let normalizedUsername = try normalize(username)
        guard let tag = previewTag(username: normalizedUsername) else {
            throw DatabaseQueryStoreError.emptyUsername
        }
        _ = try AWMDatabaseStore.saveTagIfAbsent(username: normalizedUsername, tag: tag)
        return try listTagMappings()
    }

    static func removeTagMappings(usernames: Set<String>) throws -> [TagMappingEntry] {
        guard !usernames.isEmpty else {
            return try listTagMappings()
        }

        let normalized = try Set(usernames.map(normalize))
        let deleted = try AWMDatabaseStore.removeTagMappings(usernames: Array(normalized))
        if deleted == 0, let first = normalized.first {
            throw DatabaseQueryStoreError.mappingNotFound(first)
        }
        return try listTagMappings()
    }

    static func listEvidence(limit: Int = 200) throws -> [EvidenceEntry] {
        try AWMDatabaseStore.listEvidence(limit: max(1, limit)).map { row in
            EvidenceEntry(
                id: row.id,
                createdAt: toInt64(row.createdAt),
                filePath: row.filePath,
                tag: row.tag,
                identity: row.identity,
                version: Int(row.version),
                keySlot: Int(row.keySlot),
                timestampMinutes: Int64(row.timestampMinutes),
                pcmSha256: row.pcmSha256,
                keyId: row.keyId,
                isForcedEmbed: row.isForcedEmbed,
                snrDb: row.snrDb,
                snrStatus: row.snrStatus
            )
        }
    }

    static func removeEvidence(ids: Set<Int64>) throws -> [EvidenceEntry] {
        guard !ids.isEmpty else {
            return try listEvidence()
        }
        _ = try AWMDatabaseStore.removeEvidence(ids: Array(ids))
        return try listEvidence()
    }

    static func previewTag(username: String) -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }

        if let existing = try? AWMDatabaseStore.lookupTag(username: normalized) {
            return existing
        }
        return try? AWMDatabaseStore.suggestTag(username: normalized)
    }

    private static func normalize(_ username: String) throws -> String {
        let trimmed = username.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            throw DatabaseQueryStoreError.emptyUsername
        }
        return trimmed
    }

    private static func toInt64(_ value: UInt64) -> Int64 {
        if value > UInt64(Int64.max) {
            return Int64.max
        }
        return Int64(value)
    }
}
