import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.colorScheme) private var colorScheme
    @AppStorage("appearanceMode") private var appearanceMode: AppearanceMode = .system
    @StateObject private var embedVM = EmbedViewModel()
    @StateObject private var detectVM = DetectViewModel()
    @StateObject private var keyVM = KeyViewModel()

    var body: some View {
        ZStack {
            AppBackground()
            NavigationSplitView(sidebar: {
                sidebar
            }, detail: {
                detailView
                    .background(Color.clear)
                    .navigationTitle("AWMKit")
            })
            .navigationSplitViewColumnWidth(min: 200, ideal: 240)
        }
        .background(WindowConfigurator(appState: appState))
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            List(AppState.Tab.allCases, selection: $appState.selectedTab) { tab in
                NavigationLink(value: tab) {
                    Label(appState.localizedTabTitle(tab), systemImage: tab.icon)
                }
            }
            .listStyle(.sidebar)
            .scrollIndicators(.hidden)

            Spacer(minLength: 0)

            languageSwitcher
                .padding(.horizontal, 12)
                .padding(.bottom, 10)

            appearanceSwitcher
                .padding(.horizontal, 12)
                .padding(.bottom, 16)
        }
        .frame(minWidth: 180)
    }

    private var languageSwitcher: some View {
        VStack(spacing: 6) {
            Label(localizedText("语言", "Language"), systemImage: "globe")
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)

            GlassEffectContainer {
                Picker("", selection: Binding(
                    get: { appState.uiLanguage },
                    set: { appState.setUILanguage($0) }
                )) {
                    ForEach(UILanguageOption.allCases) { language in
                        Text(language.displayName).tag(language)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .controlSize(.large)
                .glassEffect(.regular.interactive(true), in: Capsule())
                .animation(.easeInOut(duration: 0.3), value: appState.uiLanguage)
                .id(colorScheme)
            }
            .id(colorScheme)
        }
    }

    private var appearanceSwitcher: some View {
        VStack(spacing: 6) {
            Label(localizedText("外观", "Appearance"), systemImage: "circle.lefthalf.filled")
                .help(localizedText("切换应用外观", "Switch app appearance"))
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)

            GlassEffectContainer {
                Picker("", selection: $appearanceMode) {
                    ForEach(AppearanceMode.allCases) { mode in
                        Text(appearanceModeDisplayName(mode)).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .controlSize(.large)
                .glassEffect(.regular.interactive(true), in: Capsule())
                .animation(.easeInOut(duration: 0.3), value: appearanceMode)
                .id(colorScheme)
            }
            .id(colorScheme)
        }
    }

    private func appearanceModeDisplayName(_ mode: AppearanceMode) -> String {
        switch mode {
        case .system:
            return localizedText("系统", "System")
        case .light:
            return localizedText("亮色", "Light")
        case .dark:
            return localizedText("暗色", "Dark")
        }
    }

    private func localizedText(_ zh: String, _ en: String) -> String {
        appState.uiLanguage == .enUS ? en : zh
    }

    @ViewBuilder
    private var detailView: some View {
        switch appState.selectedTab {
        case .embed:
            EmbedView(viewModel: embedVM)
        case .detect:
            DetectView(viewModel: detectVM)
        case .tags:
            TagsView()
        case .key:
            KeyView(viewModel: keyVM)
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
        .frame(width: 1024, height: 680)
}
