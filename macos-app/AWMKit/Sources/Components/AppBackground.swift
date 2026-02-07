import SwiftUI

struct AppBackground: View {
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        Group {
            if colorScheme == .dark {
                LinearGradient(
                    colors: [
                        Color(red: 0.08, green: 0.09, blue: 0.11),
                        Color(red: 0.12, green: 0.13, blue: 0.15)
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )
            } else {
                LinearGradient(
                    colors: [
                        Color(red: 0.97, green: 0.98, blue: 0.995),
                        Color(red: 0.93, green: 0.95, blue: 0.98)
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )
                .overlay(
                    RadialGradient(
                        colors: [Color.white.opacity(0.9), Color.white.opacity(0.7), Color.clear],
                        center: .topLeading,
                        startRadius: 0,
                        endRadius: 550
                    )
                )
            }
        }
        .ignoresSafeArea()
    }
}
