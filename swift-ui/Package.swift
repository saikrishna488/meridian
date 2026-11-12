// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "MarsOSUI",
    products: [.library(name: "MarsOSKit", targets: ["MarsOSKit"]), .executable(name: "meridian-native-applications", targets: ["MarsOS"])],
    dependencies: [
        .package(url: "https://github.com/moreSwift/swift-cross-ui",
                 revision: "931d67ecc7c4f2551b64d68a709c60521cde0a7c")
    ],
    targets: [
        .systemLibrary(name: "CMarsOS"),
        .target(name: "MarsOSKit", dependencies: [
            "CMarsOS",
            .product(name: "SwiftCrossUI", package: "swift-cross-ui"),
            .product(name: "GtkBackend", package: "swift-cross-ui"),
            .product(name: "Gtk", package: "swift-cross-ui")
        ]),
        .executableTarget(name: "MarsOS", dependencies: [
            "CMarsOS", "MarsOSKit",
            .product(name: "SwiftCrossUI", package: "swift-cross-ui"),
            .product(name: "GtkBackend", package: "swift-cross-ui"),
            .product(name: "DefaultBackend", package: "swift-cross-ui")
        ]),
        .testTarget(name: "MarsOSTests", dependencies: ["MarsOS"])
    ]
)
