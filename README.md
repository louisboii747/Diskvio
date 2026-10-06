# Diskvio
A modern, native cross-platform disk and partition manager, written in Rust.

## macOS app

Open `macos/Diskvio/Diskvio.xcodeproj` and build the `Diskvio` scheme on an Apple Silicon Mac. Xcode's **Build Rust FFI** phase runs Cargo, creates `libdiskvio_ffi.a` for the selected architecture, and links it into the SwiftUI app. Rust and its `aarch64-apple-darwin` target must be installed. No library needs to be copied into Xcode by hand.

The read-only path is SwiftUI → `DiskService` → `diskvio_ffi.h` → `diskvio-ffi` → `diskvio-core` → macOS disk discovery. The FFI returns a JSON success or error response; Swift frees its Rust-owned string after decoding.

The build phase also maps `x86_64` to `x86_64-apple-darwin` and combines architecture libraries when Xcode requests more than one architecture. Install that Rust target with `rustup target add x86_64-apple-darwin` before building an Intel or universal app.
