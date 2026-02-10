import Foundation
import SQLite3

enum AppSettingsStoreError: LocalizedError {
    case homeDirectoryMissing
    case sqlite(String)

    var errorDescription: String? {
        switch self {
        case .homeDirectoryMissing:
            return "无法定位用户目录"
        case .sqlite(let message):
            return "设置数据库错误: \(message)"
        }
    }
}

enum AppSettingsStore {
    private static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
    private static let activeKeySlotKey = "active_key_slot"

    static func getActiveKeySlot() throws -> Int {
        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = """
            SELECT value
            FROM app_settings
            WHERE key = ?1
            LIMIT 1
            """
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            try bind(activeKeySlotKey, at: 1, in: statement, db: db)
            guard sqlite3_step(statement) == SQLITE_ROW else {
                return 0
            }
            guard let valuePtr = sqlite3_column_text(statement, 0) else {
                return 0
            }
            let value = String(cString: valuePtr)
            let parsed = Int(value) ?? 0
            return min(max(parsed, 0), 31)
        }
    }

    static func setActiveKeySlot(_ slot: Int) throws {
        let normalized = min(max(slot, 0), 31)
        let now = Int64(Date().timeIntervalSince1970)
        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }

            let sql = """
            INSERT INTO app_settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key)
            DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            """
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            try bind(activeKeySlotKey, at: 1, in: statement, db: db)
            try bind(String(normalized), at: 2, in: statement, db: db)
            guard sqlite3_bind_int64(statement, 3, now) == SQLITE_OK else {
                throw databaseError(db)
            }
            guard sqlite3_step(statement) == SQLITE_DONE else {
                throw databaseError(db)
            }
        }
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
            throw AppSettingsStoreError.sqlite(message)
        }
        defer { sqlite3_close(db) }

        try ensureSchema(in: db)
        return try body(db)
    }

    private static func ensureSchema(in db: OpaquePointer?) throws {
        let sql = """
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )
        """
        guard sqlite3_exec(db, sql, nil, nil, nil) == SQLITE_OK else {
            throw databaseError(db)
        }
    }

    private static func bind(_ string: String, at index: Int32, in statement: OpaquePointer?, db: OpaquePointer?) throws {
        guard sqlite3_bind_text(statement, index, string, -1, sqliteTransient) == SQLITE_OK else {
            throw databaseError(db)
        }
    }

    private static func databaseURL() throws -> URL {
        let home = NSHomeDirectory()
        guard !home.isEmpty else {
            throw AppSettingsStoreError.homeDirectoryMissing
        }
        return URL(fileURLWithPath: home, isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent("awmkit.db", isDirectory: false)
    }

    private static func databaseError(_ db: OpaquePointer?) -> AppSettingsStoreError {
        AppSettingsStoreError.sqlite(sqliteMessage(from: db))
    }

    private static func sqliteMessage(from db: OpaquePointer?) -> String {
        guard let db, let cString = sqlite3_errmsg(db) else {
            return "未知 SQLite 错误"
        }
        return String(cString: cString)
    }
}
