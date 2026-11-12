import SwiftCrossUI

// Shared visual primitive for the Swift GUI: generous spacing, rounded neutral
// cards and platform-native buttons preserve GTK keyboard/focus behavior.
struct ApplicationTile: View {
    let name: String
    let launch: @MainActor @Sendable () -> Void

    var body: some View {
        HStack {
            Text(name)
            Spacer()
            Button("Open", action: launch)
        }
        .padding(12)
        .background(MarsOSTheme.card)
        .cornerRadius(MarsOSTheme.cardRadius)
    }
}
