import MarsOSKit
import CMarsOS
import Foundation
import GtkBackend
import SwiftCrossUI

// C owns this callback box until the inspected GTK widget is destroyed.
private final class DesktopMenuHandler {
    let action: @MainActor @Sendable (meridian_desktop_action) -> Void
    init(_ action: @escaping @MainActor @Sendable (meridian_desktop_action) -> Void) { self.action = action }
}

@MainActor
private func attachDesktopMenu(_ widget: UnsafeMutableRawPointer,
                               action: @escaping @MainActor @Sendable (meridian_desktop_action) -> Void) {
    let context = Unmanaged.passRetained(DesktopMenuHandler(action)).toOpaque()
    let attached = meridian_desktop_menu_attach(widget, { action, context in
        guard let context else { return }
        MainActor.assumeIsolated {
            Unmanaged<DesktopMenuHandler>.fromOpaque(context).takeUnretainedValue().action(action)
        }
    }, context, { context in
        if let context { Unmanaged<DesktopMenuHandler>.fromOpaque(context).release() }
    })
    if attached == 0 { Unmanaged<DesktopMenuHandler>.fromOpaque(context).release() }
}

enum DesktopPalette {
    static let names = ["Ocean", "Forest", "Dusk"]
    static let colors = [Color(red: 0.10, green: 0.18, blue: 0.27),
                         Color(red: 0.10, green: 0.23, blue: 0.19),
                         Color(red: 0.24, green: 0.15, blue: 0.26)]
    static func color(_ index: Int) -> Color { colors[colors.indices.contains(index) ? index : 0] }
}

struct DesktopSurface: View {
    @Binding var section: String
    @Binding var directory: String
    @Binding var desktopPath: String
    let palette: Int
    let revision: Int
    @Environment(\.openWindow) private var openWindow
    @State private var entries: [FileEntry] = []
    @State private var status = ""
    @State private var busy = false

    @MainActor private func showFiles(present: OpenWindowAction) {
        directory = desktopPath
        section = "Files"
        present(id: "workspace")
    }
    @MainActor private func refresh() {
        guard !busy else { return }
        busy = true
        Task { @MainActor in
            defer { busy = false }
            do {
                if ProcessInfo.processInfo.environment["MERIDIAN_NATIVE_DESKTOP_DIRECTORY"] == nil {
                    let locations = try await nativeRequest("finder", "files.locations") as? [[String: Any]] ?? []
                    if let configured = locations.first(where: { $0["name"] as? String == "Desktop" })?["path"] as? String {
                        desktopPath = configured
                    }
                }
                let result = try await nativeRequest("finder", "files.list", ["path": desktopPath, "hidden": false]) as? [String: Any]
                let rows = result?["entries"] as? [[String: Any]] ?? []
                entries = rows.compactMap { row in
                    guard let path = row["path"] as? String, let name = row["name"] as? String,
                          let folder = row["directory"] as? Bool else { return nil }
                    return FileEntry(path: path, name: name, directory: folder)
                }
                status = entries.isEmpty ? "Your desktop is empty" : ""
            } catch { entries = []; status = error.localizedDescription }
        }
    }
    @MainActor private func action(_ action: meridian_desktop_action, present: OpenWindowAction) {
        switch action {
        case MERIDIAN_DESKTOP_FILES: showFiles(present: present)
        case MERIDIAN_DESKTOP_NEW_FOLDER: present(id: "desktop-new-folder")
        case MERIDIAN_DESKTOP_REFRESH: refresh()
        case MERIDIAN_DESKTOP_APPEARANCE: present(id: "desktop-appearance")
        case MERIDIAN_DESKTOP_SETTINGS:
            section = "Settings"
            present(id: "workspace")
        default: break
        }
    }
    var body: some View {
        let present = openWindow
        return HStack {
            VStack(spacing: 12) {
                ScrollView {
                    VStack(spacing: 8) {
                        ForEach(Array(entries.prefix(40))) { entry in
                            MarsDesktopItem(entry.name, icon: entry.directory ? "folder" : "text-x-generic") {
                                if entry.directory {
                                    directory = entry.path
                                    section = "Files"
                                    openWindow(id: "workspace")
                                } else {
                                    Task { @MainActor in
                                        do { _ = try await nativeRequest("finder", "files.open", ["path": entry.path]) }
                                        catch { status = error.localizedDescription }
                                    }
                                }
                            }
                        }
                    }
                }
                if entries.count > 40 { Button("Show all in Files") { showFiles(present: present) } }
                Text(status).foregroundColor(.white)
                Spacer()
            }.frame(width: 128)
            Spacer()
        }.padding(24)
            .padding(.top, 32)
            .padding(.bottom, 72)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(DesktopPalette.color(palette))
            .inspect { widget in
                attachDesktopMenu(UnsafeMutableRawPointer(widget.gobjectPointer)) { action($0, present: present) }
            }
            .onAppear { refresh() }
            .onChange(of: revision) { refresh() }
    }
}

struct DesktopFolderCreator: View {
    let path: String
    @Binding var revision: Int
    @Environment(\.dismissWindow) private var dismissWindow
    @State private var name = ""
    @State private var status = ""
    @State private var busy = false

    @MainActor private func create() {
        guard !busy && !name.isEmpty else { return }
        busy = true
        Task { @MainActor in
            defer { busy = false }
            do {
                _ = try await nativeRequest("finder", "files.mkdir", ["path": path, "name": name])
                revision += 1
                name = ""
                status = ""
                dismissWindow()
            } catch { status = error.localizedDescription }
        }
    }
    var body: some View {
        VStack(spacing: 16) {
            Text("New desktop folder").font(.title)
            Text(path)
            TextField("Folder name", text: $name).onSubmit { create() }
            Text(status)
            HStack {
                Button("Cancel") { dismissWindow() }
                MarsActionButton(busy ? "Creating…" : "Create", primary: true) { create() }.disabled(busy || name.isEmpty)
            }
        }.padding(24).frame(minWidth: 400, minHeight: 240).marsMaterial(.panel)
    }
}

struct DesktopAppearance: View {
    @Binding var palette: Int
    var body: some View {
        VStack(spacing: 16) {
            Text("Desktop wallpaper").font(.title)
            ForEach(Array(DesktopPalette.names.enumerated()), id: \.offset) { item in
                Button((palette == item.offset ? "✓  " : "") + item.element) { palette = item.offset }
                    .padding(8).background(DesktopPalette.color(item.offset)).cornerRadius(8)
            }
        }.padding(24).frame(minWidth: 320, minHeight: 260).marsMaterial(.panel)
    }
}
