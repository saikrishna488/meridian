import MarsOSKit
import CMarsOS
import DefaultBackend
import Foundation
import GtkBackend
import SwiftCrossUI

struct ApplicationEntry {
    let id: String
    let name: String
    let icon: String
}

// C++ owns the immutable GIO snapshot; Swift copies display strings and releases
// the snapshot exactly once. No borrowed pointer escapes this object.
struct Catalog {
    private let storage = CatalogStorage()
    var entries: [ApplicationEntry] { storage.entries }
    var error: String { storage.error }
    func launch(_ id: String) -> String { storage.launch(id) }
}

private final class CatalogStorage {
    private let handle: OpaquePointer?
    let entries: [ApplicationEntry]
    let error: String

    init() {
        handle = meridian_catalog_create()
        guard let handle else {
            entries = []
            error = "Could not load installed applications."
            return
        }
        entries = (0..<meridian_catalog_count(handle)).compactMap { index in
            guard let id = meridian_catalog_id(handle, index),
                  let name = meridian_catalog_name(handle, index) else { return nil }
            let icon = meridian_catalog_icon(handle, index).map { String(cString: $0) } ?? "application-x-executable"
            return ApplicationEntry(id: String(cString: id), name: String(cString: name), icon: icon)
        }
        error = ""
    }

    deinit { meridian_catalog_destroy(handle) }

    func launch(_ id: String) -> String {
        var error = [CChar](repeating: 0, count: 1024)
        let capacity = error.count
        let result = id.withCString { meridian_catalog_launch(handle, $0, &error, capacity) }
        return result == 1 ? "" : error.withUnsafeBufferPointer { String(cString: $0.baseAddress!) }
    }
}

@main
struct MarsOSApplications: App {
    let backend = GtkBackend(appIdentifier: "org.meridian.NativeApplications")
    @FocusState private var searchFocused: Bool
    @State private var query = ""
    @State private var status = ""
    @State private var catalog = Catalog()
    @State private var section = "Applications"
    @State private var pins = Array((UserDefaults.standard.stringArray(forKey: "nativeDockPins") ?? []).prefix(8))
    @State private var directory = ProcessInfo.processInfo.environment["MERIDIAN_NATIVE_DIRECTORY"] ?? FileManager.default.homeDirectoryForCurrentUser.path
    @State private var desktopPath = ProcessInfo.processInfo.environment["MERIDIAN_NATIVE_DESKTOP_DIRECTORY"] ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Desktop").path
    @State private var desktopRevision = 0
    @State private var palette = UserDefaults.standard.integer(forKey: "nativeDesktopPalette")
    private var sessionMode: Bool { ProcessInfo.processInfo.arguments.contains("--session") }

    private func savePreference(_ value: Any, key: String) {
        let defaults = UserDefaults.standard
        defaults.set(value, forKey: key)
        // Linux Foundation must flush before a shell process can be terminated.
        if !defaults.synchronize() {
            FileHandle.standardError.write(Data("Could not save native desktop preferences.\n".utf8))
        }
    }

    var matches: [ApplicationEntry] {
        catalog.entries.filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
    }

    var body: some Scene {
        Window("MarsOS Desktop Prototype", id: "workspace") {
            if ProcessInfo.processInfo.arguments.contains("--desktop") || sessionMode {
                DesktopPrototype(section: $section, pins: $pins, directory: $directory)
                    .marsAppearance()
                    .onChange(of: pins) { savePreference(pins, key: "nativeDockPins") }
            } else {
                VStack(spacing: 16) {
                    HStack {
                        Text("Applications").font(.title)
                        Button("Refresh") { catalog = Catalog(); status = ""; searchFocused = true }
                    }
                    TextField("Search applications", text: $query)
                        .focused($searchFocused)
                        .onSubmit {
                            if let first = matches.first { status = catalog.launch(first.id) }
                        }
                    if !catalog.error.isEmpty { Text(catalog.error) }
                    if !status.isEmpty { Text(status) }
                    if matches.isEmpty { Text("No applications found") }
                    ScrollView {
                        VStack(spacing: 8) {
                            ForEach(matches, id: \.id) { app in
                                ApplicationTile(name: app.name, icon: app.icon) { status = catalog.launch(app.id) }
                            }
                        }
                    }
                }.padding(24)
                    .background(MarsOSTheme.canvas)
                    .marsAppearance()
                    .onAppear { searchFocused = true }
                    .inspectWindow { window in
                        if ProcessInfo.processInfo.arguments.contains("--overlay") {
                            var error = [CChar](repeating: 0, count: 256)
                            let capacity = error.count
                            let configured = meridian_surface_applications(UnsafeMutableRawPointer(window.gobjectPointer), &error, capacity)
                            if configured == 0 {
                                FileHandle.standardError.write(Data("Could not configure launcher overlay.\n".utf8))
                                exit(1)
                            }
                        }
                    }
            }
        }.defaultSize(width: 960, height: 680)
            .defaultLaunchBehavior(.presented)

        Window("MarsOS wallpaper", id: "wallpaper") {
            DesktopSurface(section: $section, directory: $directory, desktopPath: $desktopPath,
                           palette: palette, revision: desktopRevision)
                .marsAppearance()
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_WALLPAPER) }
        }.defaultLaunchBehavior(sessionMode ? .presented : .suppressed)

        Window("MarsOS top bar", id: "topbar") {
            DesktopTopBar(section: $section)
                .marsAppearance()
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_TOPBAR) }
        }.defaultSize(width: 1000, height: 32)
            .defaultLaunchBehavior(sessionMode ? .presented : .suppressed)

        Window("MarsOS dock", id: "dock") {
            DesktopDock(section: $section, pins: $pins)
                .marsAppearance()
                .onChange(of: pins) { savePreference(pins, key: "nativeDockPins") }
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_DOCK) }
        }.defaultSize(width: 560, height: 72)
            .windowResizability(.contentSize)
            .defaultLaunchBehavior(sessionMode ? .presented : .suppressed)

        Window("New Folder", id: "desktop-new-folder") {
            DesktopFolderCreator(path: desktopPath, revision: $desktopRevision).marsAppearance()
        }.defaultSize(width: 440, height: 240)

        Window("Desktop Appearance", id: "desktop-appearance") {
            DesktopAppearance(palette: $palette)
                .marsAppearance()
                .onChange(of: palette) { savePreference(palette, key: "nativeDesktopPalette") }
        }.defaultSize(width: 360, height: 320)
    }
}
