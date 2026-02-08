import Foundation
import AWMKit
import CryptoKit
import SQLite3

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

    var createdDate: Date {
        Date(timeIntervalSince1970: TimeInterval(createdAt))
    }
}

enum DatabaseQueryStoreError: LocalizedError {
    case emptyUsername
    case homeDirectoryMissing
    case mappingNotFound(String)
    case sqlite(String)

    var errorDescription: String? {
        switch self {
        case .emptyUsername:
            return "用户名不能为空"
        case .homeDirectoryMissing:
            return "无法定位用户目录"
        case .mappingNotFound(let username):
            return "未找到用户映射: \(username)"
        case .sqlite(let message):
            return "数据库错误: \(message)"
        }
    }
}

enum DatabaseQueryStore {
    private static let charset = Array("ABCDEFGHJKMNPQRSTUVWXYZ23456789_")
    private static let databaseFileName = "awmkit.db"
    private static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    static func listTagMappings(limit: Int = 200) throws -> [TagMappingEntry] {
        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = """
            SELECT username, tag, created_at
            FROM tag_mappings
            ORDER BY username COLLATE NOCASE ASC
            LIMIT ?1
            """

            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            guard sqlite3_bind_int64(statement, 1, Int64(max(1, limit))) == SQLITE_OK else {
                throw databaseError(db)
            }

            var entries: [TagMappingEntry] = []
            while sqlite3_step(statement) == SQLITE_ROW {
                guard
                    let usernamePtr = sqlite3_column_text(statement, 0),
                    let tagPtr = sqlite3_column_text(statement, 1)
                else {
                    continue
                }

                let username = String(cString: usernamePtr).trimmingCharacters(in: .whitespacesAndNewlines)
                let tag = String(cString: tagPtr).uppercased()
                let createdAt = sqlite3_column_int64(statement, 2)

                guard !username.isEmpty, (try? AWMTag(tag: tag)) != nil else {
                    continue
                }

                entries.append(
                    TagMappingEntry(username: username, tag: tag, createdAt: createdAt)
                )
            }

            return entries
        }
    }

    static func saveTagMapping(username: String) throws -> [TagMappingEntry] {
        let normalizedUsername = try normalize(username)
        let tag = try AWMTag(identity: suggestedIdentity(from: normalizedUsername))
        let now = Int64(Date().timeIntervalSince1970)

        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = """
            INSERT INTO tag_mappings (username, tag, created_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(username)
            DO UPDATE SET
                username = excluded.username,
                tag = excluded.tag,
                created_at = excluded.created_at
            """

            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            try bind(normalizedUsername, at: 1, in: statement, db: db)
            try bind(tag.value, at: 2, in: statement, db: db)

            guard sqlite3_bind_int64(statement, 3, now) == SQLITE_OK else {
                throw databaseError(db)
            }
            guard sqlite3_step(statement) == SQLITE_DONE else {
                throw databaseError(db)
            }
        }

        return try listTagMappings()
    }

    static func removeTagMappings(usernames: Set<String>) throws -> [TagMappingEntry] {
        guard !usernames.isEmpty else {
            return try listTagMappings()
        }

        let normalized = try Set(usernames.map(normalize))
        var deleted = 0

        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE"
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            for username in normalized {
                sqlite3_reset(statement)
                sqlite3_clear_bindings(statement)
                try bind(username, at: 1, in: statement, db: db)
                guard sqlite3_step(statement) == SQLITE_DONE else {
                    throw databaseError(db)
                }
                deleted += Int(sqlite3_changes(db))
            }
        }

        if deleted == 0, let first = normalized.first {
            throw DatabaseQueryStoreError.mappingNotFound(first)
        }

        return try listTagMappings()
    }

    static func listEvidence(limit: Int = 200) throws -> [EvidenceEntry] {
        try withDatabase { db in
            guard try tableExists(db: db, tableName: "audio_evidence") else {
                return []
            }

            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = """
            SELECT id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes, pcm_sha256
            FROM audio_evidence
            ORDER BY created_at DESC
            LIMIT ?1
            """

            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            guard sqlite3_bind_int64(statement, 1, Int64(max(1, limit))) == SQLITE_OK else {
                throw databaseError(db)
            }

            var entries: [EvidenceEntry] = []
            while sqlite3_step(statement) == SQLITE_ROW {
                guard
                    let filePathPtr = sqlite3_column_text(statement, 2),
                    let tagPtr = sqlite3_column_text(statement, 3),
                    let identityPtr = sqlite3_column_text(statement, 4),
                    let shaPtr = sqlite3_column_text(statement, 8)
                else {
                    continue
                }

                let entry = EvidenceEntry(
                    id: sqlite3_column_int64(statement, 0),
                    createdAt: sqlite3_column_int64(statement, 1),
                    filePath: String(cString: filePathPtr),
                    tag: String(cString: tagPtr),
                    identity: String(cString: identityPtr),
                    version: Int(sqlite3_column_int(statement, 5)),
                    keySlot: Int(sqlite3_column_int(statement, 6)),
                    timestampMinutes: sqlite3_column_int64(statement, 7),
                    pcmSha256: String(cString: shaPtr)
                )
                entries.append(entry)
            }

            return entries
        }
    }

    static func removeEvidence(ids: Set<Int64>) throws -> [EvidenceEntry] {
        guard !ids.isEmpty else {
            return try listEvidence()
        }

        try withDatabase { db in
            guard try tableExists(db: db, tableName: "audio_evidence") else {
                return
            }

            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = "DELETE FROM audio_evidence WHERE id = ?1"
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            for id in ids {
                sqlite3_reset(statement)
                sqlite3_clear_bindings(statement)
                guard sqlite3_bind_int64(statement, 1, id) == SQLITE_OK else {
                    throw databaseError(db)
                }
                guard sqlite3_step(statement) == SQLITE_DONE else {
                    throw databaseError(db)
                }
            }
        }

        return try listEvidence()
    }

    static func previewTag(username: String) -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }
        return try? AWMTag(identity: suggestedIdentity(from: normalized)).value
    }

    private static func normalize(_ username: String) throws -> String {
        let trimmed = username.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            throw DatabaseQueryStoreError.emptyUsername
        }
        return trimmed
    }

    private static func suggestedIdentity(from username: String) -> String {
        let digest = SHA256.hash(data: Data(username.utf8))
        var acc: UInt64 = 0
        var accBits: UInt8 = 0
        var output = ""
        output.reserveCapacity(7)

        for byte in digest {
            acc = (acc << 8) | UInt64(byte)
            accBits += 8

            while accBits >= 5 && output.count < 7 {
                let shift = accBits - 5
                let index = Int((acc >> UInt64(shift)) & 0x1F)
                output.append(charset[index])
                accBits -= 5
            }

            if output.count >= 7 {
                break
            }
        }

        return output
    }

    private static func withDatabase<T>(_ body: (OpaquePointer?) throws -> T) throws -> T {
        let url = try databaseURL()
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )

        var db: OpaquePointer?
        guard sqlite3_open(url.path, &db) == SQLITE_OK else {
            let message = db.flatMap { sqliteMessage(from: $0) } ?? "无法打开数据库"
            if let db {
                sqlite3_close(db)
            }
            throw DatabaseQueryStoreError.sqlite(message)
        }
        defer { sqlite3_close(db) }

        try ensureTagSchema(in: db)
        return try body(db)
    }

    private static func ensureTagSchema(in db: OpaquePointer?) throws {
        let sql = """
        CREATE TABLE IF NOT EXISTS tag_mappings (
            username TEXT NOT NULL COLLATE NOCASE PRIMARY KEY,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tag_mappings_created_at
        ON tag_mappings(created_at DESC);
        """

        guard sqlite3_exec(db, sql, nil, nil, nil) == SQLITE_OK else {
            throw databaseError(db)
        }
    }

    private static func tableExists(db: OpaquePointer?, tableName: String) throws -> Bool {
        let escaped = tableName.replacingOccurrences(of: "'", with: "''")
        let sql = "SELECT 1 FROM sqlite_master WHERE type='table' AND name='\(escaped)' LIMIT 1"
        var statement: OpaquePointer?

        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
            throw databaseError(db)
        }
        defer { sqlite3_finalize(statement) }

        let step = sqlite3_step(statement)
        if step == SQLITE_ROW {
            return true
        }
        if step == SQLITE_DONE {
            return false
        }
        throw databaseError(db)
    }

    private static func bind(_ value: String, at index: Int32, in statement: OpaquePointer?, db: OpaquePointer?) throws {
        guard sqlite3_bind_text(statement, index, value, -1, sqliteTransient) == SQLITE_OK else {
            throw databaseError(db)
        }
    }

    private static func databaseError(_ db: OpaquePointer?) -> DatabaseQueryStoreError {
        .sqlite(sqliteMessage(from: db))
    }

    private static func sqliteMessage(from db: OpaquePointer?) -> String {
        guard let db, let cString = sqlite3_errmsg(db) else {
            return "unknown sqlite error"
        }
        return String(cString: cString)
    }

    private static func databaseURL() throws -> URL {
        let homePath = NSHomeDirectory()
        if homePath.isEmpty {
            throw DatabaseQueryStoreError.homeDirectoryMissing
        }
        return URL(fileURLWithPath: homePath, isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent(databaseFileName, isDirectory: false)
    }
}
