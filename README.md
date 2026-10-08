# Diskvio
A modern, native cross-platform disk and partition manager, written in Rust.

## macOS app

Open `macos/Diskvio/Diskvio.xcodeproj` and build the `Diskvio` scheme on an Apple Silicon Mac. Xcode's **Build Rust FFI** phase runs Cargo, creates `libdiskvio_ffi.a` for the selected architecture, and links it into the SwiftUI app. Rust and its `aarch64-apple-darwin` target must be installed. No library needs to be copied into Xcode by hand.

The read-only path is SwiftUI → `DiskService` → `diskvio_ffi.h` → `diskvio-ffi` → `diskvio-core` → macOS disk discovery. The FFI returns a JSON success or error response; Swift frees its Rust-owned string after decoding.

The build phase also maps `x86_64` to `x86_64-apple-darwin` and combines architecture libraries when Xcode requests more than one architecture. Install that Rust target with `rustup target add x86_64-apple-darwin` before building an Intel or universal app.

## Disk & partition explorer

The macOS app displays real physical disks, partitions, APFS containers and volumes in a native sidebar, with a proportional partition map and a resizable properties inspector. It shows available filesystem and UUID information, mounted snapshots, hardware location/connection, and storage usage. Refresh preserves selection by hierarchy identity. Disk Arbitration notifications update the hierarchy on relevant device and mount changes without polling.

Mount, Unmount, and Eject appear in the toolbar and context menus only where the Rust backend permits them. Internal disks are inspection-only. Every operation requires an explicit action, verifies current device identity, and reports success or failure. Diskvio does not format, repartition, resize, delete, write raw disks, or flash images.

For architecture, FFI ownership, operation policy and deployment details, see [macOS architecture](docs/macos-architecture.md). For validation and hardware test steps, see [macOS testing](docs/macos-testing.md).

The original CLI output is preserved. `cargo run -p diskvio-cli -- --json` prints structured topology; Windows continues to support basic physical disk discovery.
