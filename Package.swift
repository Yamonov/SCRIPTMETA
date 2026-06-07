// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ScriptMetaKit",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "ScriptMetaKit",
            targets: ["ScriptMetaKit"]
        )
    ],
    targets: [
        .binaryTarget(
            name: "ScriptMetaKitFFI",
            path: "SCRIPTMETAKit/Artifacts/ScriptMetaKitFFI.xcframework"
        ),
        .target(
            name: "ScriptMetaKit",
            dependencies: ["ScriptMetaKitFFI"],
            path: "SCRIPTMETAKit/Sources/ScriptMetaKit"
        ),
        .testTarget(
            name: "ScriptMetaKitTests",
            dependencies: ["ScriptMetaKit"],
            path: "SCRIPTMETAKit/Tests/ScriptMetaKitTests"
        )
    ]
)
