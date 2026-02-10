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
                    Label(tab.rawValue, systemImage: tab.icon)
                }
            }
            .listStyle(.sidebar)
            .scrollIndicators(.hidden)

            Spacer(minLength: 0)

            appearanceSwitcher
                .padding(.horizontal, 12)
                .padding(.top, 10)
                .padding(.bottom, 16)
        }
        .frame(minWidth: 180)
    }

    private var appearanceSwitcher: some View {
        VStack(spacing: 6) {
            Label("外观", systemImage: "circle.lefthalf.filled")
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)

            GlassEffectContainer {
                Picker("", selection: $appearanceMode) {
                    ForEach(AppearanceMode.allCases) { mode in
                        Text(mode.displayName).tag(mode)
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
