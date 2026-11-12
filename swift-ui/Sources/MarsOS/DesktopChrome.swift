import MarsOSKit
import Foundation
import SwiftCrossUI

struct DesktopTopBar: View {
    @Binding var section: String
    @Environment(\.openWindow) private var openWindow
    @State private var now = Date()
    @State private var timer: Timer?
    private func show(_ destination: String) { section = destination; openWindow(id: "workspace") }
    var body: some View {
        HStack(spacing: 20) {
            Menu("MarsOS") {
                Button("Applications") { show("Applications") }
                Button("Files") { show("Files") }
                Divider()
                Button("Settings") { show("Settings") }
                Button("Desktop appearance") { openWindow(id: "desktop-appearance") }
            }
            Text(section).fontWeight(.semibold)
            Spacer()
            Text(now.formatted(date: .abbreviated, time: .shortened))
            Button("Settings") { show("Settings") }
        }.padding(.horizontal, 12).frame(height: 32).font(.system(size: 13)).marsMaterial(.chrome)
            .onAppear {
                now = Date()
                timer?.invalidate()
                timer = Timer.scheduledTimer(withTimeInterval: 15, repeats: true) { _ in
                    Task { @MainActor in now = Date() }
                }
            }
            .onDisappear { timer?.invalidate(); timer = nil }
    }
}

struct DesktopDock: View {
    @Binding var section: String
    @Binding var pins: [String]
    @Environment(\.openWindow) private var openWindow
    @State private var catalog = Catalog()
    @State private var status = ""
    private var pinned: [ApplicationEntry] {
        pins.compactMap { id in catalog.entries.first { $0.id == id } }
    }
    var body: some View {
        VStack(spacing: 4) {
            HStack(spacing: 8) {
                MarsDockItem("Applications", icon: "view-app-grid") { section = "Applications"; openWindow(id: "workspace") }
                MarsDockItem("Files", icon: "system-file-manager") { section = "Files"; openWindow(id: "workspace") }
                MarsDockItem("Settings", icon: "preferences-system") { section = "Settings"; openWindow(id: "workspace") }
                if !pinned.isEmpty { Divider().frame(width: 1, height: 36) }
                ForEach(pinned, id: \.id) { app in
                    MarsDockItem(app.name, icon: app.icon) { status = catalog.launch(app.id) }
                }
                Menu("Dock") {
                    Button("Pin applications…") { section = "Applications"; openWindow(id: "workspace") }
                    Button("Refresh applications") { catalog = Catalog(); status = catalog.error }
                    if !pins.isEmpty { Divider() }
                    ForEach(pins, id: \.self) { id in
                        Button("Unpin " + (catalog.entries.first { $0.id == id }?.name ?? id)) { pins.removeAll { $0 == id } }
                    }
                }
            }
            if !status.isEmpty { Text(status) }
        }.padding(8).frame(width: Double(274 + pinned.count * 64)).marsMaterial(.dock)
    }
}
