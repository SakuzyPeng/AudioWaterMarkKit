import SwiftUI
import AWMKit
import CryptoKit
import SQLite3

struct TagsView: View {
    @Environment(\.colorScheme) private var colorScheme
    @State private var tags: [TagEntry] = []
    @State private var newUsername: String = ""
    @State private var showingAddSheet = false
    @State private var isDeleteMode = false
    @State private var selectedUsernames: Set<String> = []
    @State private var showingDeleteConfirm = false
    @State private var deleteConfirmInput = ""
    @State private var errorMessage: String?
    private let tagColumns = [
        GridItem(.adaptive(minimum: 180, maximum: 260), spacing: DesignSystem.Spacing.compact)
    ]

    var body: some View {
        GeometryReader { proxy in
            VStack(spacing: DesignSystem.Spacing.card) {
                // 标签列表
                GlassCard {
                    VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                        HStack {
                            Text("用户标签映射")
                                .font(.headline.weight(.semibold))

                            Spacer()

                            StatusCapsule(
                                status: "\(tags.count) 个标签",
                                isHighlight: !tags.isEmpty
                            )
                        }

                        if tags.isEmpty {
                            HStack {
                                Spacer()
                                VStack(spacing: 8) {
                                    Image(systemName: "tag")
                                        .font(.system(size: 32))
                                        .foregroundStyle(.secondary)
                                    Text("暂无标签映射")
                                        .font(.subheadline)
                                        .foregroundStyle(.secondary)
                                    Text("点击下方按钮添加用户标签")
                                        .font(.caption)
                                        .foregroundStyle(.tertiary)
                                }
                                Spacer()
                            }
                            .frame(minHeight: 120)
                        } else {
                            ScrollView {
                                LazyVGrid(columns: tagColumns, spacing: DesignSystem.Spacing.compact) {
                                    ForEach(tags, id: \.username) { entry in
                                        TagEntryRow(
                                            entry: entry,
                                            isDeleteMode: isDeleteMode,
                                            isSelected: selectedUsernames.contains(entry.username),
                                            onToggleSelected: {
                                                toggleSelection(username: entry.username)
                                            }
                                        )
                                    }
                                }
                                .padding(.vertical, 2)
                            }
                            .scrollIndicators(.hidden)
                        }
                    }
                }

                // 操作按钮
                HStack(spacing: DesignSystem.Spacing.item) {
                    if isDeleteMode {
                        Button(action: exitDeleteMode) {
                            HStack {
                                Image(systemName: "xmark.circle")
                                Text("退出删除")
                            }
                        }
                        .buttonStyle(GlassButtonStyle())
                        .accessibilityLabel("退出删除模式")

                        Button(action: selectAllTags) {
                            HStack {
                                Image(systemName: "checkmark.circle")
                                Text("全选")
                            }
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(tags.isEmpty)

                        Button(action: clearSelection) {
                            HStack {
                                Image(systemName: "circle.dashed")
                                Text("全不选")
                            }
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(selectedUsernames.isEmpty)

                        Button(action: handleDeleteAction) {
                            HStack {
                                Image(systemName: "trash")
                                Text("执行删除")
                            }
                        }
                        .buttonStyle(GlassButtonStyle(accentOn: true))
                    } else {
                        Button(action: { showingAddSheet = true }) {
                            HStack {
                                Image(systemName: "plus.circle")
                                Text("添加标签")
                            }
                        }
                        .buttonStyle(GlassButtonStyle(accentOn: true))

                        Button(action: enterDeleteMode) {
                            HStack {
                                Image(systemName: "trash")
                                Text("删除模式")
                            }
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(tags.isEmpty)
                    }

                    Spacer()
                }

                Spacer()
            }
            .padding(.horizontal, DesignSystem.Spacing.horizontal)
            .padding(.vertical, DesignSystem.Spacing.vertical)
            .frame(width: proxy.size.width, alignment: .top)
        }
        .sheet(isPresented: $showingAddSheet) {
            AddTagSheet(
                username: $newUsername,
                onSave: saveNewTag
            )
        }
        .sheet(isPresented: $showingDeleteConfirm) {
            DeleteConfirmSheet(
                expectedCount: selectedUsernames.count,
                input: $deleteConfirmInput,
                onCancel: { showingDeleteConfirm = false },
                onConfirm: {
                    showingDeleteConfirm = false
                    performBatchDelete()
                }
            )
        }
        .alert("操作失败", isPresented: .init(
            get: { errorMessage != nil },
            set: { isPresented in
                if !isPresented {
                    errorMessage = nil
                }
            }
        )) {
            Button("确定", role: .cancel) {}
        } message: {
            Text(errorMessage ?? "")
        }
        .onChange(of: tags.map(\.username)) { _, usernames in
            let available = Set(usernames)
            selectedUsernames = selectedUsernames.intersection(available)
            if usernames.isEmpty {
                exitDeleteMode()
            }
        }
        .onAppear(perform: loadTags)
    }

    private func loadTags() {
        do {
            tags = try TagStoreBridge.list()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func saveNewTag() {
        guard !newUsername.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }

        do {
            tags = try TagStoreBridge.save(username: newUsername)
            newUsername = ""
            showingAddSheet = false
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func enterDeleteMode() {
        isDeleteMode = true
        selectedUsernames.removeAll()
        deleteConfirmInput = ""
    }

    private func exitDeleteMode() {
        isDeleteMode = false
        selectedUsernames.removeAll()
        deleteConfirmInput = ""
        showingDeleteConfirm = false
    }

    private func toggleSelection(username: String) {
        guard isDeleteMode else { return }
        if selectedUsernames.contains(username) {
            selectedUsernames.remove(username)
        } else {
            selectedUsernames.insert(username)
        }
    }

    private func selectAllTags() {
        selectedUsernames = Set(tags.map(\.username))
    }

    private func clearSelection() {
        selectedUsernames.removeAll()
    }

    private func handleDeleteAction() {
        guard isDeleteMode else { return }
        if selectedUsernames.isEmpty {
            exitDeleteMode()
            return
        }
        deleteConfirmInput = ""
        showingDeleteConfirm = true
    }

    private func performBatchDelete() {
        do {
            tags = try TagStoreBridge.remove(usernames: selectedUsernames)
            exitDeleteMode()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

struct TagEntry {
    let username: String
    let tag: String
}

struct TagEntryRow: View {
    let entry: TagEntry
    let isDeleteMode: Bool
    let isSelected: Bool
    let onToggleSelected: () -> Void

    var body: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            Image(systemName: "person.circle.fill")
                .foregroundStyle(.secondary)
                .font(.system(size: 17, weight: .medium))

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.username)
                    .font(.subheadline.weight(.medium))
                    .lineLimit(1)

                Text(entry.tag)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 6)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .entryRowStyle()
        .overlay {
            if isDeleteMode && isSelected {
                RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous)
                    .stroke(Color.accentColor.opacity(0.9), lineWidth: 1.5)
            }
        }
        .overlay(alignment: .topTrailing) {
            if isDeleteMode && isSelected {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
                    .padding(6)
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: DesignSystem.CornerRadius.row, style: .continuous))
        .onTapGesture {
            if isDeleteMode {
                onToggleSelected()
            }
        }
    }
}

struct AddTagSheet: View {
    @Binding var username: String
    let onSave: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var suggestedTag: String? {
        TagStoreBridge.previewTag(username: username)
    }

    var body: some View {
        VStack(spacing: DesignSystem.Spacing.card) {
            Text("添加标签映射")
                .font(.title2.weight(.semibold))

            VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("用户名")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)

                    GlassEffectContainer {
                        TextField("例如: SakuzyPeng", text: $username)
                            .textFieldStyle(.plain)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                    }
                    .background(DesignSystem.Colors.rowBackground(colorScheme))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                    )
                }

                VStack(alignment: .leading, spacing: 6) {
                    Text("自动生成 Tag")
                        .font(.subheadline)
                        .foregroundStyle(.secondary.opacity(0.85))

                    GlassEffectContainer {
                        HStack {
                            Text(suggestedTag ?? "-")
                                .font(.system(.body, design: .monospaced).weight(.semibold))
                                .foregroundStyle(suggestedTag == nil ? .tertiary : .primary)
                            Spacer()
                        }
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                    }
                    .background(DesignSystem.Colors.rowBackground(colorScheme))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                    )

                    Text("基于用户名稳定生成（预览即最终保存值）")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }

            HStack(spacing: DesignSystem.Spacing.item) {
                Button("取消") {
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button("保存") {
                    onSave()
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(username.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(30)
        .frame(width: 420)
    }
}

struct DeleteConfirmSheet: View {
    let expectedCount: Int
    @Binding var input: String
    let onCancel: () -> Void
    let onConfirm: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

    private var trimmedInput: String {
        input.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var isValid: Bool {
        Int(trimmedInput) == expectedCount
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DesignSystem.Spacing.card) {
            Text("确认删除")
                .font(.title3.weight(.semibold))

            VStack(alignment: .leading, spacing: 8) {
                Text("此操作不可恢复，请输入数量确认。")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Text("我确认删除 \(expectedCount) 条标签")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.primary)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("请输入数字：\(expectedCount)")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                GlassEffectContainer {
                    TextField("输入 \(expectedCount)", text: $input)
                        .textFieldStyle(.plain)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                }
                .background(DesignSystem.Colors.rowBackground(colorScheme))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(DesignSystem.Colors.border(colorScheme), lineWidth: DesignSystem.BorderWidth.standard)
                )
            }

            HStack(spacing: DesignSystem.Spacing.item) {
                Button("取消") {
                    onCancel()
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle())

                Button("确认删除") {
                    onConfirm()
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(!isValid)
            }
        }
        .padding(24)
        .frame(width: 420)
    }
}

private enum TagStoreBridgeError: LocalizedError {
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
            return "标签数据库错误: \(message)"
        }
    }
}

private enum TagStoreBridge {
    private static let charset = Array("ABCDEFGHJKMNPQRSTUVWXYZ23456789_")
    private static let databaseFileName = "awmkit.db"
    private static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    static func list() throws -> [TagEntry] {
        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }
            let sql = """
            SELECT username, tag
            FROM tag_mappings
            ORDER BY username COLLATE NOCASE ASC
            """
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            var entries: [TagEntry] = []
            while sqlite3_step(statement) == SQLITE_ROW {
                guard
                    let usernamePtr = sqlite3_column_text(statement, 0),
                    let tagPtr = sqlite3_column_text(statement, 1)
                else {
                    continue
                }

                let username = String(cString: usernamePtr).trimmingCharacters(in: .whitespacesAndNewlines)
                let tag = String(cString: tagPtr).uppercased()
                guard !username.isEmpty, (try? AWMTag(tag: tag)) != nil else {
                    continue
                }
                entries.append(TagEntry(username: username, tag: tag))
            }
            return entries
        }
    }

    static func save(username: String) throws -> [TagEntry] {
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
        return try list()
    }

    static func previewTag(username: String) -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }
        return try? AWMTag(identity: suggestedIdentity(from: normalized)).value
    }

    static func remove(username: String) throws -> [TagEntry] {
        let normalizedUsername = try normalize(username)
        let affected = try withDatabase { db -> Int32 in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }
            let sql = "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE"
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }
            try bind(normalizedUsername, at: 1, in: statement, db: db)
            guard sqlite3_step(statement) == SQLITE_DONE else {
                throw databaseError(db)
            }
            return sqlite3_changes(db)
        }
        if affected == 0 {
            throw TagStoreBridgeError.mappingNotFound(normalizedUsername)
        }
        return try list()
    }

    static func clear() throws -> [TagEntry] {
        try withDatabase { db in
            guard sqlite3_exec(db, "DELETE FROM tag_mappings", nil, nil, nil) == SQLITE_OK else {
                throw databaseError(db)
            }
        }
        return []
    }

    static func remove(usernames: Set<String>) throws -> [TagEntry] {
        guard !usernames.isEmpty else {
            return try list()
        }
        try withDatabase { db in
            var statement: OpaquePointer?
            defer { sqlite3_finalize(statement) }
            let sql = "DELETE FROM tag_mappings WHERE username = ?1 COLLATE NOCASE"
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw databaseError(db)
            }

            for username in usernames {
                sqlite3_reset(statement)
                sqlite3_clear_bindings(statement)
                try bind(username, at: 1, in: statement, db: db)
                guard sqlite3_step(statement) == SQLITE_DONE else {
                    throw databaseError(db)
                }
            }
        }
        return try list()
    }

    private static func normalize(_ username: String) throws -> String {
        let trimmed = username.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            throw TagStoreBridgeError.emptyUsername
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
            throw TagStoreBridgeError.sqlite(message)
        }
        defer { sqlite3_close(db) }

        try ensureSchema(in: db)
        return try body(db)
    }

    private static func ensureSchema(in db: OpaquePointer?) throws {
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

    private static func bind(_ value: String, at index: Int32, in statement: OpaquePointer?, db: OpaquePointer?) throws {
        guard sqlite3_bind_text(statement, index, value, -1, sqliteTransient) == SQLITE_OK else {
            throw databaseError(db)
        }
    }

    private static func databaseError(_ db: OpaquePointer?) -> TagStoreBridgeError {
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
            throw TagStoreBridgeError.homeDirectoryMissing
        }
        return URL(fileURLWithPath: homePath, isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent(databaseFileName, isDirectory: false)
    }
}
