# Windows architecture

## Scope and repository contract

Diskvio's Windows application is a native WinUI 3 desktop frontend over the existing Rust backend. All discovery remains in `diskvio-core`; C# does not enumerate disks independently. This milestone performs inspection only, as the current Windows backend exposes basic `Get-Disk` discovery.

The repository inspected for this milestone contains `Disk`, not `DiskInventory`, and exports only `diskvio_list_disks_json` and `diskvio_string_free`. It does **not** yet contain `diskvio_inventory_json`, `diskvio_operation_json`, partition records or an operation policy. The Windows app binds only the exports that actually exist. It does not add pretend inventory/operation APIs or infer partitions from physical disk capacity.

No macOS Swift, Xcode, platform discovery or operation behavior is changed. The Rust crate retains its `staticlib` for Xcode and `rlib` for Rust consumers and additionally builds a `cdylib` for Windows.

## Solution and project structure

```text
windows/
  Diskvio.slnx                         # Debug/Release, explicit x64 mapping
  build/Diskvio.Rust.targets           # shared Cargo build and native content
  Diskvio.Windows/
    Diskvio.Windows.csproj            # official packaged WinUI template
    App.xaml[.cs]                     # resources and window lifetime
    MainWindow.xaml[.cs]              # TitleBar, Mica, NavigationView shell
    Views/DevicesPage.xaml[.cs]        # native controls and initial load
    ViewModels/DevicesViewModel.cs     # state, refresh and selection
    Models/                           # exact Rust JSON fields + presentation
    Services/                         # async discovery, decoder, P/Invoke
    Converters/                       # view-only visibility conversion
    Assets/                           # packaged app icons and tile assets
    Package.appxmanifest               # desktop package, no elevation
    Properties/launchSettings.json    # packaged Visual Studio launch
  Diskvio.Windows.Tests/              # contract/state + native integration tests
```

The application was scaffolded with Microsoft's `Microsoft.WindowsAppSDK.WinUI.CSharp.Templates` package and `dotnet new winui`. The standard SDK-style project and single-project MSIX tooling are retained. The scaffold package installed here was version `0.0.7-alpha`; its **WinUI Blank App** template is the official packaged XAML template, and the application runtime references stable packages, not an experimental WinUI/Reactor runtime.

The app uses `net10.0-windows10.0.26100.0`, minimum Windows build 19041, `win-x64`, and x64 compilation. `Diskvio.slnx` explicitly maps both solution configurations to x64 and enables deployment for the app. ARM64 and x86 are not offered or supported by the build pipeline.

Pinned dependencies:

- `Microsoft.WindowsAppSDK` **2.5.1** (stable).
- `Microsoft.Windows.SDK.BuildTools` **10.0.28000.2705** (build tooling; distinct from the target Windows API SDK).
- `Microsoft.Windows.SDK.BuildTools.WinApp` **0.7.1** (the official template's CLI package-identity launch integration).
- `CommunityToolkit.Mvvm` **8.4.2** (`ObservableObject` and `AsyncRelayCommand`).

The app follows the system theme; all surfaces and text use Fluent theme resources. Mica is enabled only when `MicaController.IsSupported()`. The native window initially requests 1280 × 780 pixels. At wide sizes, the disk list and details sit beside each other; at narrow sizes they stack, with independent scrolling. State triggers live on the page's root layout panel. Standard ListView/Button navigation, accessible names, selectable identifiers, live status regions and Ctrl+R/F5 refresh provide keyboard and assistive-technology access. No custom animation timeline is added.

## Rust DLL build and package inclusion

`crates/diskvio-ffi/Cargo.toml` declares `crate-type = ["staticlib", "cdylib", "rlib"]`. Rust's existing `#[unsafe(no_mangle)] pub extern "C"` functions become DLL exports without changing their names or ABI.

`windows/build/Diskvio.Rust.targets` runs before `PrepareForBuild` on real builds (not design-time builds):

```text
cargo build --locked --manifest-path <repo>/Cargo.toml
  --package diskvio-ffi --target x86_64-pc-windows-msvc
  --target-dir <repo>/target/windows [--release]
```

Debug selects `debug`; Release selects `release` and `--release`. Cargo handles incremental compilation and errors fail the MSBuild build. The target verifies the architecture/configuration and DLL's existence. `DiskvioCargoExecutable` defaults to the user's usual Cargo path if installed there, otherwise `cargo` on PATH, and can be overridden with an MSBuild property. Visual Studio's fast up-to-date bypass is disabled for the app, so edits to Rust sources still reach Cargo even when managed sources have not changed.

The expected DLL is declared as a `Content` item **before it exists**, linked as `diskvio_ffi.dll`, with `CopyToOutputDirectory` and `CopyToPublishDirectory`. This lets the ordinary WinUI/MSIX pipeline include it in build output, publish output and the package root. The include has no `Exists` condition that could drop the DLL on a clean build. A Cargo failure has no stale-DLL fallback. No manual copy step is required.

The tests import the same target, so their native integration test also receives the DLL automatically. Generated package artifacts are local development output. An unsigned MSIX used to inspect package contents is not a distributable release: signing and distribution credentials remain a separate release task.

## FFI JSON and memory ownership

Success:

```json
{"status":"ok","disks":[{"number":0,"name":"<backend-reported name>","size_bytes":0,"bus_type":"<backend-reported bus>","partition_style":"<backend-reported scheme>"}]}
```

Error:

```json
{"status":"error","message":"<backend error>"}
```

These are schema illustrations, not production fixtures. `number` is Rust `u32` / C# `uint`; `size_bytes` is Rust `u64` / C# `ulong`. All three text fields are strings. The C# models use explicit `JsonPropertyName` attributes and required members, and the decoder enforces nullable annotations. Missing fields, unknown statuses, negative/out-of-range numbers, null entries and duplicate disk numbers are errors. An empty `disks` array is valid and results in the empty state. New JSON fields can be introduced additively in Rust without breaking this reader.

`diskvio_list_disks_json()` returns a null pointer on encoding/allocation-response failure or a Rust-owned, NUL-terminated UTF-8 `CString`. Every non-null allocation belongs to Rust and must be returned once to `diskvio_string_free(pointer)`. Free accepts null. The C# declarations use exact entry points and `CallingConvention.Cdecl`; the return value is a `RustStringHandle`, not an automatically marshalled managed string.

Before making the allocation call, `NativeMethods` loads **the absolute DLL path in `AppContext.BaseDirectory`** and verifies both list and free exports. A missing DLL, incompatible image or missing export becomes an explicit `DiskServiceException`; there is no alternative backend or fabricated inventory. The loaded library remains alive for the process lifetime so SafeHandle finalizers can call its free function safely.

`DiskService` owns the handle in a `using` scope, copies the NUL-terminated bytes with a 16 MiB response limit, strictly decodes UTF-8, and parses JSON into managed models. Disposal invokes Rust's free exactly once on success or any decoding/error path. The CLR never releases this memory using `FreeHGlobal`, `FreeCoTaskMem` or another allocator. The private handle has one owner and cannot be disposed concurrently during the copy. Managed records retain only copied values, never native pointers.

## Data flow and refresh behavior

```text
DevicesPage -> DevicesViewModel.RefreshCommand
  -> DiskService.ListDisksAsync -> Task.Run
  -> P/Invoke diskvio_list_disks_json
  -> diskvio-ffi -> diskvio-core::list_disks
  -> cfg(windows) backend -> system Windows PowerShell -> Get-Disk
  -> UTF-8 JSON -> SafeHandle copy/decode/dispose
  -> managed disk records -> UI observable collection / selection
```

Enumeration, process waiting, copying and decoding run on a worker thread. The view model awaits the service and resumes on WinUI's synchronization context before changing its collection. `AsyncRelayCommand` prevents overlapping refreshes. Disk selection is retained by number when that number remains present; otherwise the first remaining disk is selected. Loading disables inspection and refresh. Empty, loading and error states are distinct. Failed refreshes discard old results so disconnected devices are not presented as current inventory.

The legacy Windows disk-list path still uses `Get-Disk`. Rich inventory additionally uses structured `Get-Partition` and `Get-Volume` output. For desktop use, it launches the system PowerShell executable with `CREATE_NO_WINDOW`, drains stdout/stderr concurrently, and terminates/reaps discovery after a 30-second timeout. Process, parsing and command failures cross the existing JSON error envelope. No discovery command requests elevation or writes storage. The existing `IDiskService` also exposes inventory, capability queries, validation and operation execution through the same SafeHandle-owned JSON boundary. Storage errors preserve structured backend codes in `DiskServiceException.BackendError`.

`\\.\PhysicalDrive<number>` is the Windows device path derived from the backend's disk number; C# does not open it. Disk numbers are not durable hardware identities across unplug/replug or reboot, so refresh selection is only best effort. Capacity formatting uses decimal units matching the existing frontends, with exact bytes also displayed.

## Current limitations

- Only x64 Windows discovery/inspection is implemented.
- Rust and the C# service expose `DiskInventory`, partitions, volumes, filesystems, drive letters, mount paths, available space and safety flags. The WinUI page still consumes the legacy physical-disk list and does not render those details yet.
- The service exposes validated mount/unmount for supported external USB volumes, but no WinUI operation controls are enabled yet. Windows eject is unsupported. Format, create/delete/resize, image writing, privileged helpers and elevation remain absent.
- The Windows minimum OS declaration is build 19041; visual/runtime validation for this milestone is on Windows 11, not a Windows 10 compatibility matrix.
- macOS SwiftUI/Xcode tests must run on a Mac; building/testing the Rust workspace on Windows does not execute `cfg(macos)` tests.
- Refresh is explicit; automatic device-change notifications and cancellation beyond the native timeout are future work.

## Validation and future extension points

The managed test project compiles the actual UI-independent model/service/view-model source as linked files. This avoids loading a WinUI executable into the test host while testing the same implementation, without introducing a second production architecture. Fixtures exist only in tests. Tests verify UTF-8 names, `u64` capacity, malformed/error responses, selection after refresh/removal, stale-data clearing, empty/loading states and overlap prevention. The `NativeIntegration` test enumerates real disks twice through the built DLL; environmental discovery failures fail the test rather than silently switching to fixtures.

Milestone 2 provides the richer Rust inventory, compatible JSON exports, typed C# models and service APIs. The remaining Windows work is an inventory view and capability-driven action controls, followed by native DLL/WinUI/MSIX and hardware verification. Bind the existing UI to the service APIs without inferring capabilities or using a drive letter as device identity. See [Milestone 2 backend](milestone-2-backend.md) for request modes, safety rules and exact remaining work. Future ARM64 support requires a matching Rust target, build mapping, runtime assets and validation; adding an architecture label alone is insufficient.

Official references: [WinUI quickstart and templates](https://learn.microsoft.com/windows/apps/get-started/start-here), [stable Windows App SDK downloads](https://learn.microsoft.com/windows/apps/windows-app-sdk/downloads), [single-project MSIX](https://learn.microsoft.com/windows/apps/windows-app-sdk/single-project-msix), [responsive XAML layouts](https://learn.microsoft.com/windows/apps/develop/ui/layouts-with-xaml).
