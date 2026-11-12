import CMarsOS
import DefaultBackend
import Foundation
import GtkBackend
import SwiftCrossUI

struct ApplicationEntry {
    let id: String
    let name: String
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
            return ApplicationEntry(id: String(cString: id), name: String(cString: name))
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
    private var sessionMode: Bool { ProcessInfo.processInfo.arguments.contains("--session") }

    var matches: [ApplicationEntry] {
        catalog.entries.filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
    }

    var body: some Scene {
        Window("MarsOS Desktop Prototype", id: "workspace") {
            if ProcessInfo.processInfo.arguments.contains("--desktop") || sessionMode {
                DesktopPrototype(section: $section)
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
                                ApplicationTile(name: app.name) { status = catalog.launch(app.id) }
                            }
                        }
                    }
                }.padding(24)
                    .background(MarsOSTheme.canvas)
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
            VStack {
                Spacer()
                Text("MarsOS").font(.title)
                Text("Swift desktop preview")
                Spacer()
            }.frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color(red: 0.10, green: 0.18, blue: 0.27))
                .foregroundColor(.white)
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_WALLPAPER) }
        }.defaultLaunchBehavior(sessionMode ? .presented : .suppressed)

        Window("MarsOS top bar", id: "topbar") {
            HStack {
                Text("MarsOS").font(.title)
                Spacer()
                Text("Native desktop prototype")
                NativeNavigation(section: $section)
            }.padding(8).background(MarsOSTheme.card)
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_TOPBAR) }
        }.defaultSize(width: 1000, height: 48)
            .defaultLaunchBehavior(sessionMode ? .presented : .suppressed)

        Window("MarsOS dock", id: "dock") {
            NativeNavigation(section: $section)
                .padding(12).background(MarsOSTheme.card)
                .inspectWindow { configureSurface(UnsafeMutableRawPointer($0.gobjectPointer), MERIDIAN_SURFACE_DOCK) }
        }.defaultSize(width: 340, height: 64)
            .defaultLaunchBehavior(sessionMode ? .presented : .suppressed)
    }
}
