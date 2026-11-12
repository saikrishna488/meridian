import CMarsOS
import Foundation
import GtkBackend
import SwiftCrossUI

struct NativeNavigation: View {
    @Binding var section: String
    @Environment(\.openWindow) private var openWindow
    var body: some View {
        HStack(spacing: 12) {
            ForEach(["Applications", "Files", "Settings"], id: \.self) { name in
                Button(name) { section = name; openWindow(id: "workspace") }
            }
        }
    }
}

@MainActor
func configureSurface(_ pointer: UnsafeMutableRawPointer, _ role: meridian_surface_role) {
    var error = [CChar](repeating: 0, count: 256)
    let capacity = error.count
    if meridian_surface_configure(pointer, role, &error, capacity) == 0 {
        let message = error.withUnsafeBufferPointer { String(cString: $0.baseAddress!) }
        FileHandle.standardError.write(Data("Native surface: \(message)\n".utf8))
        exit(1)
    }
}
