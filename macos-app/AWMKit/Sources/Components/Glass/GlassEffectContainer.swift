import SwiftUI

struct GlassEffectContainer<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    @ViewBuilder var content: Content

    var body: some View {
        content
            .id(colorScheme)
    }
}
