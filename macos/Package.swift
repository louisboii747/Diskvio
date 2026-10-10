// swift-tools-version: 6.2
import PackageDescription

// Portable presentation/contract checks. The native application remains an Xcode target.
let package = Package(
    name: "DiskvioPresentation",
    products: [.library(name: "DiskvioPresentation", targets: ["DiskvioPresentation"])],
    targets: [
        .target(name: "DiskvioPresentation", path: "Diskvio/Diskvio/Models"),
        .testTarget(name: "DiskvioPresentationTests", dependencies: ["DiskvioPresentation"], path: "Tests/PresentationTests")
    ]
)
