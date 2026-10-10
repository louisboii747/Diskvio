# Windows architecture

The native Windows frontend extends the existing .NET 10 / WinUI 3 application and x64 Visual Studio 2026 solution. [Milestone 5 Windows refinement](milestone-5-windows-refinement.md) describes the current implementation, contracts, validation and limitations; [Milestone 4](milestone-4-windows.md) records the original Windows foundation.

## Application structure

- `MainWindow`: native title bar, conditional Mica and NavigationView shell.
- `DevicesPage`: Fluent controls, disk sidebar, explorer, partition map, inspector, confirmations and a 15-second UI-thread timer for asynchronous read-only refresh.
- `StorageWorkspaceViewModel`: selection, inventory, capabilities, busy state and guarded operations. Existing `DevicesViewModel` remains for legacy discovery bindings and tests.
- `StorageNode`: presentation over existing typed storage contracts; does not invent device data.
- `PartitionPresentation`: friendly naming from labels and known backend partition types.
- `PartitionLayout`: portable, conservative physical geometry and minimum visual widths; `PartitionMap` renders theme-aware native focusable buttons, with the synchronized table as an alternative inspection target.
- `DiskService` / `NativeMethods`: existing asynchronous JSON FFI and Rust-owned SafeHandle results.
- `Diskvio.Rust.targets`: existing locked Cargo build, architecture checks and native content in build/publish/MSIX output.

## Boundary and ownership

The UI calls `diskvio_inventory_json` and `diskvio_operation_json` for inventory, capability queries, validation and supported operations. Legacy `diskvio_list_disks_json` stays available. Only Rust performs Windows discovery or volume operations. C# never invokes PowerShell, DiskPart or raw disk APIs for disk management.

Native declarations use exact C entry points, Cdecl and pointer-sized request lengths. Requests are UTF-8 JSON byte arrays borrowed for the call. Responses are Rust-owned UTF-8 strings copied and strictly decoded with a 16 MiB limit, then returned exactly once to `diskvio_string_free` through SafeHandle disposal. The resolver loads only the absolute application-local DLL, checks all four exports and holds it for process lifetime. Typed models preserve nullable metadata and unsigned byte sizes; structured backend error codes remain available on `DiskServiceException`.

The x64 solution builds `x86_64-pc-windows-msvc` Rust code into `target/windows`, maps Debug/Release profiles and packages the correct DLL automatically. Existing package versions and MSIX architecture are retained. No backend contracts, Rust implementation or macOS sources changed in Milestone 4.

## Safety and concurrency

Inventory and native calls run on worker threads. View-model state resumes on the UI synchronization context. Busy state covers discovery and the entire confirmation/validation/execution sequence; refresh and selection controls are disabled while busy. Selection-version checks discard stale capability responses. Failed inventory clears stale devices. Successful operations refresh automatically; failed operations also attempt refresh to handle disconnects or changed state.

Only an external USB basic-data volume with a backend identity and matching advertised and freshly queried capability can enable mount/unmount, label rename or drive-letter assignment/change. A single-volume partition can resolve to that volume; ambiguous partitions require explicit volume selection. Paging-file volumes and missing safety metadata are excluded. Confirmation defaults to Cancel. After confirmation, capabilities and identity are checked again, validation is requested, and Rust execution revalidates independently. New operations also require matching confirmed access paths. Drive-letter changes require an exclusive native volume lock and attempt rollback if reassignment fails. Create/delete, formatting, physical eject and privilege bypass remain unavailable.

## Validation boundaries

Tests link the actual portable models, services and view models without loading WinUI. Native integration tests perform discovery and capability queries only. Mutation tests use fake APIs and command runners; no real storage mutation runs automatically. Existing macOS fixture tests also run on Windows using a test-only plist dependency. Native macOS builds and runtime checks require a Mac. See the milestone report for exact successful commands and outstanding manual checks.
