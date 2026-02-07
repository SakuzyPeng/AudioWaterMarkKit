import SwiftUI
import AWMKit

struct TagsView: View {
    @Environment(\.colorScheme) private var colorScheme
    @State private var tags: [TagEntry] = []
    @State private var newUsername: String = ""
    @State private var newTagIdentity: String = ""
    @State private var showingAddSheet = false

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
                tagIdentity: $newTagIdentity,
                onSave: saveNewTag
            )
        }
        .onAppear(perform: loadTags)
    }

    private func loadTags() {
        // TODO: 从存储加载标签（使用 AWMKit 的 TagStore）
        tags = []
    }

    private func saveNewTag() {
        guard !newUsername.isEmpty else { return }

        do {
            let tag = try AWMTag(identity: newTagIdentity.isEmpty ? newUsername : newTagIdentity)
            tags.append(TagEntry(username: newUsername, tag: tag.value))
            newUsername = ""
            newTagIdentity = ""
            showingAddSheet = false
        } catch {
            print("创建标签失败: \(error)")
        }
    }

    private func removeTag(username: String) {
        tags.removeAll { $0.username == username }
    }

    private func clearAllTags() {
        tags.removeAll()
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
    @Binding var tagIdentity: String
    let onSave: () -> Void
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme

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
                    Text("标签（可选）")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)

                    GlassEffectContainer {
                        TextField("留空自动生成", text: $tagIdentity)
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

                    Text("7 个字符，留空则使用用户名前 7 位")
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
                    dismiss()
                }
                .buttonStyle(GlassButtonStyle(accentOn: true))
                .disabled(username.isEmpty)
            }
        }
        .padding(30)
        .frame(width: 400)
    }
}
