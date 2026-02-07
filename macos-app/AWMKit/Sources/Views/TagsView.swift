import SwiftUI
import AWMKit
import CryptoKit

struct TagsView: View {
    @Environment(\.colorScheme) private var colorScheme
    @State private var tags: [TagEntry] = []
    @State private var newUsername: String = ""
    @State private var showingAddSheet = false
    @State private var errorMessage: String?

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
                                LazyVStack(spacing: DesignSystem.Spacing.compact) {
                                    ForEach(tags, id: \.username) { entry in
                                        TagEntryRow(entry: entry) {
                                            removeTag(username: entry.username)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 操作按钮
                HStack(spacing: DesignSystem.Spacing.item) {
                    Button(action: { showingAddSheet = true }) {
                        HStack {
                            Image(systemName: "plus.circle")
                            Text("添加标签")
                        }
                    }
                    .buttonStyle(GlassButtonStyle(accentOn: true))

                    if !tags.isEmpty {
                        Button(action: clearAllTags) {
                            HStack {
                                Image(systemName: "trash")
                                Text("清空所有")
                            }
                        }
                        .buttonStyle(GlassButtonStyle())
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

    private func removeTag(username: String) {
        do {
            tags = try TagStoreBridge.remove(username: username)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func clearAllTags() {
        do {
            tags = try TagStoreBridge.clear()
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
    @Environment(\.colorScheme) private var colorScheme
    let entry: TagEntry
    let onRemove: () -> Void

    var body: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            Image(systemName: "person.circle.fill")
                .foregroundStyle(.secondary)
                .font(.title3)

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.username)
                    .font(.subheadline.weight(.medium))

                Text(entry.tag)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Button(action: onRemove) {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
        }
        .entryRowStyle()
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

private struct StoredTagEntry: Codable {
    let username: String
    let tag: String
    let createdAt: UInt64

    enum CodingKeys: String, CodingKey {
        case username
        case tag
        case createdAt = "created_at"
    }
}

private struct StoredTagPayload: Codable {
    var version: UInt8
    var entries: [StoredTagEntry]

    init(version: UInt8 = 1, entries: [StoredTagEntry] = []) {
        self.version = version
        self.entries = entries
    }
}

private enum TagStoreBridgeError: LocalizedError {
    case emptyUsername
    case homeDirectoryMissing
    case mappingNotFound(String)

    var errorDescription: String? {
        switch self {
        case .emptyUsername:
            return "用户名不能为空"
        case .homeDirectoryMissing:
            return "无法定位用户目录"
        case .mappingNotFound(let username):
            return "未找到用户映射: \(username)"
        }
    }
}

private enum TagStoreBridge {
    private static let charset = Array("ABCDEFGHJKMNPQRSTUVWXYZ23456789_")

    static func list() throws -> [TagEntry] {
        try loadPayload().entries
            .sorted(by: { $0.username.localizedCaseInsensitiveCompare($1.username) == .orderedAscending })
            .map { TagEntry(username: $0.username, tag: $0.tag) }
    }

    static func save(username: String) throws -> [TagEntry] {
        let normalizedUsername = try normalize(username)
        let tag = try AWMTag(identity: suggestedIdentity(from: normalizedUsername))

        var payload = try loadPayload()
        let now = UInt64(Date().timeIntervalSince1970)

        if let index = payload.entries.firstIndex(where: { $0.username == normalizedUsername }) {
            payload.entries[index] = StoredTagEntry(
                username: normalizedUsername,
                tag: tag.value,
                createdAt: now
            )
        } else {
            payload.entries.append(StoredTagEntry(
                username: normalizedUsername,
                tag: tag.value,
                createdAt: now
            ))
        }

        payload.version = 1
        payload.entries.sort { $0.username.localizedCaseInsensitiveCompare($1.username) == .orderedAscending }
        try persist(payload)
        return payload.entries.map { TagEntry(username: $0.username, tag: $0.tag) }
    }

    static func previewTag(username: String) -> String? {
        let normalized = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return nil }
        return try? AWMTag(identity: suggestedIdentity(from: normalized)).value
    }

    static func remove(username: String) throws -> [TagEntry] {
        let normalizedUsername = try normalize(username)
        var payload = try loadPayload()
        let before = payload.entries.count
        payload.entries.removeAll { $0.username == normalizedUsername }
        if payload.entries.count == before {
            throw TagStoreBridgeError.mappingNotFound(normalizedUsername)
        }
        try persist(payload)
        return payload.entries.map { TagEntry(username: $0.username, tag: $0.tag) }
    }

    static func clear() throws -> [TagEntry] {
        let url = try tagsFileURL()
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
        return []
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

    private static func loadPayload() throws -> StoredTagPayload {
        let url = try tagsFileURL()
        if !FileManager.default.fileExists(atPath: url.path) {
            return StoredTagPayload()
        }

        let raw = try String(contentsOf: url, encoding: .utf8)
        if raw.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return StoredTagPayload()
        }
        return try JSONDecoder().decode(StoredTagPayload.self, from: Data(raw.utf8))
    }

    private static func persist(_ payload: StoredTagPayload) throws {
        let url = try tagsFileURL()
        let directory = url.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(payload)
        try data.write(to: url, options: .atomic)
    }

    private static func tagsFileURL() throws -> URL {
        let homePath = NSHomeDirectory()
        if homePath.isEmpty {
            throw TagStoreBridgeError.homeDirectoryMissing
        }
        return URL(fileURLWithPath: homePath, isDirectory: true)
            .appendingPathComponent(".awmkit", isDirectory: true)
            .appendingPathComponent("tags.json", isDirectory: false)
    }
}
