// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Nous",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "Nous", targets: ["Nous"]),
    ],
    dependencies: [
        .package(url: "https://github.com/nicklockwood/SwiftFormat", from: "0.54.0"),
    ],
    targets: [
        .target(
            name: "Nous",
            path: "Nous/Sources"
        ),
        .testTarget(
            name: "NousTests",
            dependencies: ["Nous"],
            path: "NousTests"
        ),
    ]
)
