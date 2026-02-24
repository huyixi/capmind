// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "CapMindApple",
    defaultLocalization: "en",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "CapMindCore", targets: ["CapMindCore"]),
        .library(name: "CapMindData", targets: ["CapMindData"]),
        .library(name: "CapMindFeatures", targets: ["CapMindFeatures"]),
        .library(name: "CapMindUI", targets: ["CapMindUI"]),
        .executable(name: "CapMindApp", targets: ["CapMindApp"]),
    ],
    dependencies: [
        .package(url: "https://github.com/supabase/supabase-swift.git", from: "2.0.0"),
    ],
    targets: [
        .target(
            name: "CapMindCore"
        ),
        .target(
            name: "CapMindData",
            dependencies: [
                "CapMindCore",
                .product(name: "Supabase", package: "supabase-swift"),
            ],
            linkerSettings: [
                .linkedLibrary("sqlite3"),
            ]
        ),
        .target(
            name: "CapMindFeatures",
            dependencies: ["CapMindCore", "CapMindData"]
        ),
        .target(
            name: "CapMindUI",
            dependencies: ["CapMindCore", "CapMindFeatures"]
        ),
        .executableTarget(
            name: "CapMindApp",
            dependencies: ["CapMindCore", "CapMindData", "CapMindFeatures", "CapMindUI"]
        ),
        .testTarget(
            name: "CapMindCoreTests",
            dependencies: ["CapMindCore"]
        ),
        .testTarget(
            name: "CapMindDataTests",
            dependencies: ["CapMindCore", "CapMindData"]
        ),
    ]
)
