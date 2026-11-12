import CMarsOS
import Foundation

// All blocking C ABI work runs on this actor, never on GTK's main thread.
// Each surface has a separate capability-scoped client owned by this runtime.
actor NativeService {
    static let shared = NativeService()
    private let storage: ServiceStorage
    init(configDirectory: String? = nil) { storage = ServiceStorage(configDirectory: configDirectory) }

    func call(surface: String, request: Data) throws -> Data {
        guard let backend = storage.backend else { throw ServiceError("Native backend could not start.") }
        let client: OpaquePointer
        if let existing = storage.clients[surface] { client = existing }
        else {
            guard let created = meridian_client_create(backend, surface) else {
                throw ServiceError("Surface is unavailable.")
            }
            storage.clients[surface] = created
            client = created
        }
        let task = request.withUnsafeBytes { bytes in
            meridian_client_submit(client, bytes.baseAddress?.assumingMemoryBound(to: CChar.self), bytes.count)
        }
        guard let task else { throw ServiceError("Could not submit request.") }
        defer { meridian_task_destroy(task) }
        // Poll so cancellation and deadlines never strand the actor indefinitely.
        let deadline = Date().addingTimeInterval(15)
        while meridian_task_wait(task, 50) == 0 {
            if Task.isCancelled || Date() >= deadline {
                meridian_task_cancel(task)
                throw ServiceError("Request cancelled or timed out.")
            }
        }
        try Task.checkCancellation()
        guard let reply = meridian_task_reply(task) else { throw ServiceError("Missing service reply.") }
        return Data(String(cString: reply).utf8)
    }
}

struct ServiceError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

@MainActor
func nativeRequest(_ surface: String, _ method: String, _ params: [String: Any]? = nil, service: NativeService = .shared) async throws -> Any {
    var envelope: [String: Any] = ["method": method]
    if let params { envelope["params"] = params }
    let request = try JSONSerialization.data(withJSONObject: envelope)
    let data = try await service.call(surface: surface, request: request)
    guard let reply = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw ServiceError("Invalid service reply.")
    }
    guard reply["ok"] as? Bool == true else {
        let error = reply["error"] as? [String: Any]
        throw ServiceError(error?["message"] as? String ?? "Service unavailable.")
    }
    return reply["result"] ?? NSNull()
}

// The actor exclusively owns this storage. Its destructor runs only after the
// last actor reference is released, with no concurrent calls to the C handles.
private final class ServiceStorage {
    let backend: OpaquePointer?
    init(configDirectory: String?) {
        if let configDirectory {
            backend = configDirectory.withCString { meridian_backend_create($0) }
        } else { backend = meridian_backend_create(nil) }
    }
    var clients: [String: OpaquePointer] = [:]
    deinit {
        for client in clients.values { meridian_client_destroy(client) }
        meridian_backend_destroy(backend)
    }
}
