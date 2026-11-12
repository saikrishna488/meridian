import Foundation
import XCTest
@testable import MarsOS

final class NativeServiceTests: XCTestCase {
    @MainActor
    func testFilesSettingsAndCapabilityBoundary() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let service = NativeService(configDirectory: root.appendingPathComponent("config").path)
        let files = root.appendingPathComponent("files")
        try FileManager.default.createDirectory(at: files, withIntermediateDirectories: true)
        try Data("Swift native preview".utf8).write(to: files.appendingPathComponent("note.txt"))
        _ = try await nativeRequest("finder", "files.mkdir", ["path": files.path, "name": "folder"], service: service)
        let listing = try await nativeRequest("finder", "files.list", ["path": files.path, "hidden": false], service: service) as? [String: Any]
        let entries = listing?["entries"] as? [[String: Any]]
        XCTAssertEqual(entries?.count, 2)
        XCTAssertEqual(entries?.first?["name"] as? String, "folder")
        let preview = try await nativeRequest("finder", "files.preview", ["path": files.appendingPathComponent("note.txt").path], service: service) as? [String: Any]
        XCTAssertEqual(preview?["content"] as? String, "Swift native preview")
        do {
            _ = try await nativeRequest("finder", "settings.search_set", ["settings": false, "calculator": true], service: service)
            XCTFail("Finder must not change settings")
        } catch { XCTAssertFalse(error.localizedDescription.isEmpty) }
        _ = try await nativeRequest("preferences", "settings.search_set", ["settings": false, "calculator": true], service: service)
        let reopened = NativeService(configDirectory: root.appendingPathComponent("config").path)
        let saved = try await nativeRequest("preferences", "settings.search", service: reopened) as? [String: Any]
        XCTAssertEqual(saved?["settings"] as? Bool, false)
        XCTAssertEqual(saved?["calculator"] as? Bool, true)
        do {
            _ = try await nativeRequest("invented", "shell.hello", service: service)
            XCTFail("Unknown surface must fail")
        } catch { XCTAssertFalse(error.localizedDescription.isEmpty) }
    }
}
