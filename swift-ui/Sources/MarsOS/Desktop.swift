import Foundation
import SwiftCrossUI

struct DesktopPrototype: View {
    @Binding var section: String
    @FocusState private var navigationFocused: Bool
    var body: some View {
        VStack(spacing: 16) {
            HStack {
                Text("MarsOS").font(.title)
                Spacer()
                Button("Applications") { section = "Applications" }.focused($navigationFocused)
                Button("Files") { section = "Files" }
                Button("Settings") { section = "Settings" }
            }
            if section == "Applications" { ApplicationsPane() }
            else if section == "Files" { FilesPane() }
            else { SettingsPane() }
            Spacer()
            Text("Swift / GTK4 • C++ services • Desktop prototype")
        }.padding(24).background(MarsOSTheme.canvas).onAppear { navigationFocused = true }
    }
}

struct ApplicationsPane: View {
    @State private var catalog = Catalog()
    @State private var query = ""
    @State private var status = ""
    var matches: [ApplicationEntry] {
        catalog.entries.filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) }
    }
    var body: some View {
        VStack(spacing: 12) {
            HStack {
                TextField("Search applications", text: $query).onSubmit {
                    if let first = matches.first { status = catalog.launch(first.id) }
                }
                Button("Refresh") { catalog = Catalog(); status = "" }
            }
            Text(catalog.error.isEmpty ? status : catalog.error)
            if matches.isEmpty { Text("No applications found") }
            ScrollView {
                VStack(spacing: 8) {
                    ForEach(matches, id: \.id) { app in
                        ApplicationTile(name: app.name) { status = catalog.launch(app.id) }
                    }
                }
            }
        }
    }
}

struct FileEntry: Identifiable {
    let path: String
    let name: String
    let directory: Bool
    var id: String { path }
}

struct FilesPane: View {
    @State private var path = ProcessInfo.processInfo.environment["MERIDIAN_NATIVE_DIRECTORY"] ?? FileManager.default.homeDirectoryForCurrentUser.path
    @State private var entries: [FileEntry] = []
    @State private var preview = ""
    @State private var status = ""
    @State private var busy = false
    @State private var hidden = false
    @State private var folderName = ""

    @MainActor private func load(_ destination: String) {
        guard !busy else { return }
        busy = true
        Task { @MainActor in
            defer { busy = false }
            do {
                let result = try await nativeRequest("finder", "files.list", ["path": destination, "hidden": hidden])
                guard let listing = result as? [String: Any], let rows = listing["entries"] as? [[String: Any]],
                      let resolved = listing["path"] as? String else { throw ServiceError("Invalid directory response.") }
                entries = rows.compactMap { row in
                    guard let path = row["path"] as? String, let name = row["name"] as? String,
                          let directory = row["directory"] as? Bool else { return nil }
                    return FileEntry(path: path, name: name, directory: directory)
                }
                path = resolved
                preview = ""
                status = listing["truncated"] as? Bool == true ? "Showing first 5,000 entries" : "\(entries.count) items"
            } catch { status = error.localizedDescription }
        }
    }

    @MainActor private func perform(_ method: String, _ params: [String: Any], refresh: Bool) {
        guard !busy else { return }
        busy = true
        Task { @MainActor in
            do {
                _ = try await nativeRequest("finder", method, params)
                busy = false
                if refresh { folderName = ""; load(path) }
                else { status = "Opened with default application" }
            } catch { busy = false; status = error.localizedDescription }
        }
    }

    @MainActor private func inspect(_ entry: FileEntry) {
        if entry.directory { load(entry.path); return }
        guard !busy else { return }
        busy = true
        Task { @MainActor in
            defer { busy = false }
            do {
                let result = try await nativeRequest("finder", "files.preview", ["path": entry.path]) as? [String: Any]
                preview = result?["content"] as? String ?? "Preview unavailable for this file type."
                status = entry.name + (result?["truncated"] as? Bool == true ? " (preview truncated)" : "")
            } catch { status = error.localizedDescription }
        }
    }

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                Button("Home") { load(FileManager.default.homeDirectoryForCurrentUser.path) }
                Button("Up") { load(URL(fileURLWithPath: path).deletingLastPathComponent().path) }
                TextField("Absolute path", text: $path).onSubmit { load(path) }
                Button("Go") { load(path) }
            }
            HStack {
                Toggle("Show hidden files", isOn: $hidden)
                Button("Refresh") { load(path) }
                Text(busy ? "Loading…" : status)
            }
            HStack {
                TextField("New folder name", text: $folderName)
                Button("Create folder") { perform("files.mkdir", ["path": path, "name": folderName], refresh: true) }
            }
            ScrollView {
                VStack(spacing: 8) {
                    ForEach(entries) { entry in
                        HStack {
                            Text((entry.directory ? "Folder: " : "") + entry.name)
                            Spacer()
                            Button(entry.directory ? "Open" : "Preview") { inspect(entry) }
                            if !entry.directory {
                                Button("Open externally") { perform("files.open", ["path": entry.path], refresh: false) }
                            }
                        }.padding(8).background(MarsOSTheme.card).cornerRadius(8)
                    }
                    if !preview.isEmpty { Text(preview) }
                }
            }
        }.onAppear { load(path) }
    }
}

struct SettingsPane: View {
    @State private var includeSettings = true
    @State private var calculator = false
    @State private var status = ""
    @State private var system: [String] = []
    @State private var busy = false
    @State private var loaded = false

    @MainActor private func refresh() {
        guard !busy else { return }
        busy = true
        Task { @MainActor in
            defer { busy = false }
            do {
                guard let search = try await nativeRequest("preferences", "settings.search") as? [String: Any] else {
                    throw ServiceError("Invalid settings response.")
                }
                includeSettings = search["settings"] as? Bool ?? true
                calculator = search["calculator"] as? Bool ?? false
                loaded = true
                let info = try await nativeRequest("preferences", "settings.system_info") as? [String: Any] ?? [:]
                system = ["os", "hostname", "architecture", "kernel", "memory"].compactMap { key in
                    (info[key] as? String).map { "\(key): \($0)" }
                }
                status = "Settings loaded"
            } catch { status = error.localizedDescription }
        }
    }

    var body: some View {
        VStack(spacing: 16) {
            Text("Search preferences").font(.title)
            Toggle("Include settings in search", isOn: $includeSettings)
            Toggle("Enable calculator in search", isOn: $calculator)
            Text("These preferences are saved for the desktop search service.")
            HStack {
                Button("Reload") { refresh() }
                Button("Save") {
                    if loaded && !busy {
                        busy = true
                        Task { @MainActor in
                            defer { busy = false }
                            do {
                                _ = try await nativeRequest("preferences", "settings.search_set", ["settings": includeSettings, "calculator": calculator])
                                status = "Saved"
                            } catch { status = error.localizedDescription }
                        }
                    }
                }
            }
            Text(busy ? "Working…" : status)
            Text("About this system").font(.title)
            ForEach(system, id: \.self) { line in Text(line) }
        }.onAppear { refresh() }
    }
}
