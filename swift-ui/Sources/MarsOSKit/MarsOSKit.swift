// Adapted from Luminare's surface/button/section design architecture.
// Luminare © 2024 Kai, Kami and Jace, BSD-3-Clause; see Licenses/.
// Linux implementation: SwiftCrossUI + GTK; no Apple SwiftUI/AppKit dependency.
import CMarsOS
import Foundation
import Gtk
import GtkBackend
import SwiftCrossUI

public enum MarsTheme {
    public static let canvas = Color.adaptive(light: Color(white: 0.98), dark: Color(white: 0.125))
    public static let card = Color.adaptive(light: .white, dark: Color(white: 0.16))
    public static let accent = Color(red: 0.333, green: 0.325, blue: 0.808)
    public static let text = Color.adaptive(light: Color(white: 0.13), dark: Color(white: 0.95))
    public static let secondary = Color.adaptive(light: Color(white: 0.39), dark: Color(white: 0.72))
    public static let cardRadius = 12
    public static let controlRadius = 8
    public static let dockRadius = 20
}

public enum MarsMaterial: Sendable {
    case chrome, dock, section, panel
    fileprivate var role: meridian_style_role {
        switch self {
        case .chrome: MERIDIAN_STYLE_CHROME
        case .dock: MERIDIAN_STYLE_DOCK
        case .section: MERIDIAN_STYLE_SECTION
        case .panel: MERIDIAN_STYLE_PANEL
        }
    }
}

public extension View {
    func marsMaterial(_ material: MarsMaterial) -> some View {
        inspect { _ = meridian_widget_style(UnsafeMutableRawPointer($0.gobjectPointer), material.role) }
    }
}

public struct MarsSymbol: GtkWidgetRepresentable {
    let icon: String
    let pixels: Int
    public init(_ icon: String, size: Int = 20) { self.icon = icon; pixels = size }
    public func makeGtkWidget(context: Context) -> Gtk.Image { Gtk.Image() }
    public func updateGtkWidget(_ image: Gtk.Image, context: Context) {
        icon.withCString { meridian_image_icon(UnsafeMutableRawPointer(image.gobjectPointer), $0, Int32(pixels)) }
    }
}

public struct MarsDockItem: View {
    let title: String
    let icon: String
    let action: @MainActor @Sendable () -> Void
    public init(_ title: String, icon: String, action: @escaping @MainActor @Sendable () -> Void) {
        self.title = title; self.icon = icon; self.action = action
    }
    public var body: some View {
        Button(action: action) {
            MarsSymbol(icon, size: 44).frame(width: 44, height: 44).padding(6)
        }.inspect { button in
            let pointer = UnsafeMutableRawPointer(button.gobjectPointer)
            _ = meridian_widget_style(pointer, MERIDIAN_STYLE_DOCK_ITEM)
            title.withCString { meridian_widget_describe(pointer, $0) }
        }.buttonStyle(.plain)
    }
}

public struct MarsActionButton: View {
    let title: String
    let primary: Bool
    let action: @MainActor @Sendable () -> Void
    public init(_ title: String, primary: Bool = false, action: @escaping @MainActor @Sendable () -> Void) {
        self.title = title; self.primary = primary; self.action = action
    }
    public var body: some View {
        Button(action: action) {
            Text(title).foregroundColor(primary ? .white : MarsTheme.text)
                .padding(.horizontal, 12).padding(.vertical, 7)
        }
            .inspect { _ = meridian_widget_style(UnsafeMutableRawPointer($0.gobjectPointer), primary ? MERIDIAN_STYLE_PRIMARY : MERIDIAN_STYLE_CONTROL) }
            .buttonStyle(.plain)
    }
}

public struct MarsSection<Content: View>: View {
    let title: String
    let content: Content
    public init(_ title: String, @ViewBuilder content: () -> Content) { self.title = title; self.content = content() }
    public var body: some View {
        VStack(spacing: 8) {
            HStack { Text(title).font(.system(size: 13)); Spacer() }.foregroundColor(MarsTheme.secondary)
            content.padding(12).marsMaterial(.section)
        }
    }
}

public struct MarsDesktopItem: View {
    let title: String
    let icon: String
    let action: @MainActor @Sendable () -> Void
    public init(_ title: String, icon: String, action: @escaping @MainActor @Sendable () -> Void) {
        self.title = title; self.icon = icon; self.action = action
    }
    public var body: some View {
        Button(action: action) {
            VStack(spacing: 5) {
                MarsSymbol(icon, size: 44).frame(width: 44, height: 44)
                Text(title).font(.system(size: 12)).foregroundColor(.white)
            }.frame(width: 112, height: 78)
        }.inspect { button in
            let pointer = UnsafeMutableRawPointer(button.gobjectPointer)
            _ = meridian_widget_style(pointer, MERIDIAN_STYLE_DESKTOP_ITEM)
            title.withCString { meridian_widget_describe(pointer, $0) }
        }.buttonStyle(.plain)
    }
}

public extension View {
    /// Keep Swift tokens aligned with an explicit GTK theme override. Otherwise
    /// follow SwiftCrossUI's desktop-portal appearance, including live changes.
    func marsAppearance() -> some View { MarsAppearance(content: self) }
}

private struct MarsAppearance<Content: View>: View {
    @Environment(\.colorScheme) private var systemScheme
    let content: Content
    var body: some View {
        let environment = ProcessInfo.processInfo.environment
        let preference = environment["MERIDIAN_NATIVE_APPEARANCE"]
        let theme = environment["GTK_THEME"] ?? ""
        let scheme: ColorScheme
        if preference == "dark" || (preference == nil && theme.hasSuffix(":dark")) { scheme = .dark }
        else if preference == "light" || (preference == nil && theme.hasSuffix(":light")) { scheme = .light }
        else { scheme = systemScheme }
        return content.colorScheme(scheme)
            .inspect([.onCreate, .afterUpdate]) { widget in
                meridian_widget_appearance(UnsafeMutableRawPointer(widget.gobjectPointer), scheme == .dark ? 1 : 0)
            }
    }
}
