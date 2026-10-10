# Diskvio
A modern, native cross-platform disk and partition manager, written in Rust.

## Windows app

The Windows frontend is a native **C# / .NET 10 / WinUI 3** desktop app, using the stable Windows App SDK **2.5.1** and single-project MSIX packaging. It shares `diskvio-core` and `diskvio-ffi` with the macOS app. Windows support is currently **x64 only**.

### Prerequisites

- Windows 11 recommended (minimum Windows 10 build 19041).
- Visual Studio 2026 with **WinUI application development**, .NET 10, Windows SDK **10.0.26100.0**, and the MSVC C++ build tools required by Rust. Use the Visual Studio Installer to add missing components.
- Rust's MSVC toolchain and target: `rustup target add x86_64-pc-windows-msvc`.
- Enable Windows **Developer Mode** for local packaged deployment. Restart Visual Studio after installing Rust so it picks up Cargo. The build also detects `%USERPROFILE%\.cargo\bin\cargo.exe`.
- NuGet and crates.io access for the first restore/build.

### Open and launch in Visual Studio 2026

1. Open **`windows/Diskvio.slnx`** from this repository.
2. Set **Diskvio.Windows** as the startup project.
3. Select **Debug | x64**, and the **Diskvio.Windows (Package)** launch profile. In Configuration Manager, keep **Build** and **Deploy** checked for the app; the test project only needs Build.
4. Press **F5**. Visual Studio restores NuGet packages, builds the Rust DLL, builds/deploys the development package, and opens Diskvio. No DLL copying or administrator launch is required.
5. Choose **Refresh** (also **Ctrl+R** or **F5** inside the app) to enumerate again. Select a disk to inspect its details. **Release | x64** is also supported.

Build from a **Developer PowerShell for VS 2026**, starting at the repository root:

```powershell
dotnet restore windows/Diskvio.slnx -p:Platform=x64
msbuild windows/Diskvio.slnx /p:Configuration=Debug /p:Platform=x64
msbuild windows/Diskvio.slnx /p:Configuration=Release /p:Platform=x64
dotnet run --project windows/Diskvio.Windows/Diskvio.Windows.csproj -p:Platform=x64 --no-launch-profile
```

The official template's `Microsoft.Windows.SDK.BuildTools.WinApp` package enables `dotnet run` with package identity. Launch through Visual Studio or this command; running the EXE directly does not provide its required package identity. Close a running development instance before rebuilding/redeploying its package layout.

The MSBuild integration builds `diskvio_ffi.dll` for `x86_64-pc-windows-msvc` in `target/windows/`, maps Debug/Release to the matching Cargo profile, and includes the DLL in output, publish, and MSIX content. `DiskvioCargoExecutable` can override Cargo's path when necessary. Generated .NET, Cargo and package outputs are ignored by Git.

### Capabilities and validation

The app displays **real physical disks, partitions and volumes** discovered through the Rust inventory API, with a proportional partition layout and a detailed inspector. It shows backend-reported capacities, filesystem, drive letters, mount locations and safety metadata. Refresh preserves selection using identifiers and available stable identity; a 15-second read-only poll detects device changes. Capacity uses decimal units.

Mount, Unmount, Rename label and Drive letter are enabled only for external USB basic-data volumes with matching backend capabilities, verified identity and complete safety metadata. Every mutation requires a preview/confirmation and fresh backend validation. Windows physical eject and partition creation/deletion remain unsupported. No format, image writing or elevation controls exist. See [Milestone 5 Windows refinement](docs/milestone-5-windows-refinement.md) for contracts, safety rules, validation limits and safe manual USB checks; [Milestone 4](docs/milestone-4-windows.md) records the original Windows foundation.

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
dotnet test windows/Diskvio.Windows.Tests/Diskvio.Windows.Tests.csproj -p:Platform=x64
```

The managed tests cover legacy and richer JSON contracts, structured operation errors and refresh state, and include read-only native integration tests that enumerate this machine's disks and topology and query capabilities. To run only deterministic tests, append `--filter 'Category!=NativeIntegration'`. Test fixtures are confined to the test project. On macOS or Linux, run the same deterministic tests with `dotnet test windows/Diskvio.Windows.Tests/Diskvio.Windows.Tests.csproj -p:DiskvioPortableTests=true --filter 'Category!=NativeIntegration'`; this omits the Windows DLL build and native test.

See [Windows architecture](docs/windows-architecture.md) for the native ownership contract, build pipeline, limitations and extension points.

## macOS app

Open `macos/Diskvio/Diskvio.xcodeproj` and build the `Diskvio` scheme on an Apple Silicon Mac. Xcode's **Build Rust FFI** phase runs Cargo, creates `libdiskvio_ffi.a` for the selected architecture, and links it into the SwiftUI app. Rust and its `aarch64-apple-darwin` target must be installed. No library needs to be copied into Xcode by hand.

The read-only path is SwiftUI → `DiskService` → `diskvio_ffi.h` → `diskvio-ffi` → `diskvio-core` → macOS disk discovery. The FFI returns a JSON success or error response; Swift frees its Rust-owned string after decoding.

The build phase also maps `x86_64` to `x86_64-apple-darwin` and combines architecture libraries when Xcode requests more than one architecture. Install that Rust target with `rustup target add x86_64-apple-darwin` before building an Intel or universal app.

## Disk & partition explorer

The macOS app displays real physical disks, partitions, APFS containers and volumes in a native sidebar, with a capacity-weighted partition map, synchronized partition list and a resizable properties inspector. It shows available filesystem and UUID information, mounted snapshots, hardware location/connection, and storage usage. Refresh preserves selection by hierarchy identity. Disk Arbitration notifications update the hierarchy on relevant device and mount changes without polling.

Rename Volume, Mount, Unmount, and Eject appear in the toolbar and context menus only where the Rust backend permits them. Renaming uses a backend-validated preview and explicit confirmation; mounted external APFS/HFS/FAT/exFAT volumes require verified identity and writability. Internal disks are inspection-only. Every operation requires an explicit action, verifies current device identity, and reports success or failure. Diskvio does not format, repartition, resize, delete, write raw disks, or flash images.

The supplied artwork in `assets/` now populates the macOS AppIcon catalogue and Windows executable/package icons. Reproduce exports with `python assets/export_icons.py` (Pillow required). See [macOS Milestone 5](docs/milestone-5-macos-refinement.md) for scope, validation limits and safe external-drive checks. Portable presentation tests run with `swift test --package-path macos`.

For architecture, FFI ownership, operation policy and deployment details, see [macOS architecture](docs/macos-architecture.md). For validation and hardware test steps, see [macOS testing](docs/macos-testing.md).

The original CLI output is preserved. `cargo run -p diskvio-cli -- --json` prints structured topology; Windows inventory now includes partitions and volumes with separate device identities.
