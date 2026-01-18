// swift-tools-version:5.9
import PackageDescription
import Foundation

// 获取 Package.swift 所在目录的绝对路径
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let libPath = "\(packageDir)/../../target/release"

let package = Package(
    name: "AWMKit",
    platforms: [
        .macOS(.v12),
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "AWMKit",
            targets: ["AWMKit"]
        ),
    ],
    targets: [
        // C bridging module
        .target(
            name: "CAWMKit",
            dependencies: [],
            path: "Sources/CAWMKit",
            publicHeadersPath: "include",
            linkerSettings: [
                .unsafeFlags(["-L\(libPath)"]),
                .linkedLibrary("awmkit"),
            ]
        ),
        // Swift wrapper
        .target(
            name: "AWMKit",
            dependencies: ["CAWMKit"],
            path: "Sources/AWMKit"
        ),
        // Tests
        .testTarget(
            name: "AWMKitTests",
            dependencies: ["AWMKit"],
            path: "Tests/AWMKitTests"
        ),
    ]
)
