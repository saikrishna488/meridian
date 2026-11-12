import MarsOSKit
import SwiftCrossUI

// Shared visual primitive for the Swift GUI: generous spacing, rounded neutral
// cards and platform-native buttons preserve GTK keyboard/focus behavior.
struct ApplicationTile: View {
    let name: String
    var icon: String = "application-x-executable"
    let launch: @MainActor @Sendable () -> Void

    var body: some View {
        HStack(spacing: 12) {
            MarsSymbol(icon, size: 32).frame(width: 32, height: 32)
            Text(name)
            Spacer()
            MarsActionButton("Open", action: launch)
        }
        .padding(12)
        .marsMaterial(.section)
    }
}
