import SwiftUI
import AWMKit

struct StatusView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.colorScheme) private var colorScheme
    @State private var audiowmarkVersion: String = "检查中..."

    var body: some View {
        GeometryReader { proxy in
            VStack(spacing: DesignSystem.Spacing.card) {
                // 应用信息
                GlassCard {
                    VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                        Text("应用信息")
                            .font(.headline.weight(.semibold))

                        VStack(spacing: DesignSystem.Spacing.compact) {
                            StatusInfoRow(label: "版本", value: "0.1.0", icon: "app.badge")
                            StatusInfoRow(label: "构建", value: "Debug", icon: "hammer")
                            StatusInfoRow(label: "平台", value: "macOS 26.0+", icon: "desktopcomputer")
                        }
                    }
                }

                // 系统状态
                GlassCard {
                    VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                        Text("系统状态")
                            .font(.headline.weight(.semibold))

                        VStack(spacing: DesignSystem.Spacing.compact) {
                            HStack(spacing: DesignSystem.Spacing.item) {
                                Image(systemName: "key.fill")
                                    .foregroundStyle(.secondary)
                                    .frame(width: 20)

                                Text("密钥")
                                    .font(.subheadline)

                                Spacer()

                                StatusCapsule(
                                    status: appState.keyLoaded ? "已配置" : "未配置",
                                    isHighlight: appState.keyLoaded
                                )
                            }
                            .entryRowStyle()

                            HStack(spacing: DesignSystem.Spacing.item) {
                                Image(systemName: "waveform")
                                    .foregroundStyle(.secondary)
                                    .frame(width: 20)

                                Text("AudioWmark")
                                    .font(.subheadline)

                                Spacer()

                                StatusCapsule(
                                    status: audiowmarkVersion,
                                    isHighlight: appState.audio != nil
                                )
                            }
                            .entryRowStyle()
                        }
                    }
                }

                // 操作区
                GlassCard {
                    VStack(alignment: .leading, spacing: DesignSystem.Spacing.section) {
                        Text("操作")
                            .font(.headline.weight(.semibold))

                        HStack(spacing: DesignSystem.Spacing.item) {
                            if !appState.keyLoaded {
                                Button(action: initializeKey) {
                                    HStack {
                                        Image(systemName: "key.fill")
                                        Text("初始化密钥")
                                    }
                                }
                                .buttonStyle(GlassButtonStyle(accentOn: true))
                            }

                            Button(action: checkStatus) {
                                HStack {
                                    Image(systemName: "arrow.clockwise")
                                    Text("刷新状态")
                                }
                            }
                            .buttonStyle(GlassButtonStyle())
                        }
                    }
                }

                Spacer()
            }
            .padding(.horizontal, DesignSystem.Spacing.horizontal)
            .padding(.vertical, DesignSystem.Spacing.vertical)
            .frame(width: proxy.size.width, alignment: .top)
        }
        .onAppear(perform: checkStatus)
    }

    private func checkStatus() {
        Task {
            await appState.checkKey()
            if appState.audio != nil {
                audiowmarkVersion = "0.6.5 (bundled)"
            } else {
                audiowmarkVersion = "未找到"
            }
        }
    }

    private func initializeKey() {
        Task {
            do {
                _ = try appState.keychain.generateAndSaveKey()
                await appState.checkKey()
            } catch {
                print("密钥生成失败: \(error.localizedDescription)")
            }
        }
    }
}

struct StatusInfoRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let label: String
    let value: String
    let icon: String

    var body: some View {
        HStack(spacing: DesignSystem.Spacing.item) {
            Image(systemName: icon)
                .foregroundStyle(.secondary)
                .frame(width: 20)

            Text(label)
                .font(.subheadline)
                .foregroundStyle(.secondary)

            Spacer()

            Text(value)
                .font(.system(.subheadline, design: .monospaced))
                .foregroundStyle(.primary)
        }
        .entryRowStyle()
    }
}
