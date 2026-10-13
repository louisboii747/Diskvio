# Windows architecture

The native Windows frontend extends the existing .NET 10 / WinUI 3 application and x64 Visual Studio 2026 solution. [Milestone 5 Windows refinement](milestone-5-windows-refinement.md) describes the current implementation, contracts, validation and limitations; [Milestone 4](milestone-4-windows.md) records the original Windows foundation.

## Application structure

- `MainWindow`: native title bar, conditional Mica and NavigationView shell.
- `DevicesPage`: Fluent controls, disk sidebar, explorer, partition map, inspector, confirmations and a 15-second UI-thread timer for asynchronous read-only refresh.
- `StorageWorkspaceViewModel`: selection, inventory, capabilities, busy state and guarded operations. Existing `DevicesViewModel` remains for legacy discovery bindings and tests.
- `StorageNode`: presentation over existing typed storage contracts; does not invent device data.
- `PartitionPresentation`: friendly naming from labels and known backend partition types.
- `PartitionLayout`: conservative physical presentation, backend-reported unallocated regions, minimum visual widths and separate proposal maps. `PartitionMap` renders native focusable buttons synchronized with the explorer.
- `PartitionPlanningView`: inline creation/deletion preview editor, exact size/start inputs, session test-media designation and before/after maps. Its state belongs to the existing workspace view model.
- `DiskService` / `NativeMethods`: existing asynchronous JSON FFI and Rust-owned SafeHandle results.
- `Diskvio.Rust.targets`: existing locked Cargo build, architecture checks and native content in build/publish/MSIX output.

## Boundary and ownership

The UI calls `diskvio_inventory_json` and `diskvio_operation_json` for inventory, capability queries, validation and supported operations. Legacy `diskvio_list_disks_json` stays available. Only Rust performs Windows discovery or volume operations. C# never invokes PowerShell, DiskPart or raw disk APIs for disk management.

Native declarations use exact C entry points, Cdecl and pointer-sized request lengths. Requests are UTF-8 JSON byte arrays borrowed for the call. Responses are Rust-owned UTF-8 strings copied and strictly decoded with a 16 MiB limit, then returned exactly once to `diskvio_string_free` through SafeHandle disposal. The resolver loads only the absolute application-local DLL, checks all four exports and holds it for process lifetime. Typed models preserve nullable metadata and unsigned byte sizes; structured backend error codes remain available on `DiskServiceException`.

The x64 solution builds `x86_64-pc-windows-msvc` Rust code into `target/windows`, maps Debug/Release profiles and packages the correct DLL automatically. Existing package versions and MSIX architecture are retained. No backend contracts, Rust implementation or macOS sources changed in Milestone 4.

## Safety and concurrency

Inventory and native calls run on worker threads. View-model state resumes on the UI synchronization context. Busy state covers discovery and the entire confirmation/validation/execution sequence; refresh and selection controls are disabled while busy. Selection-version checks discard stale capability responses. Failed inventory clears stale devices. Successful operations refresh automatically; failed operations also attempt refresh to handle disconnects or changed state.

Only an external USB basic-data volume with a backend identity and matching advertised and freshly queried capability can enable mount/unmount, label rename or drive-letter assignment/change. A single-volume partition can resolve to that volume; ambiguous partitions require explicit volume selection. Paging-file volumes and missing safety metadata are excluded. Confirmation defaults to Cancel. After confirmation, capabilities and identity are checked again, validation is requested, and Rust execution revalidates independently. New operations also require matching confirmed access paths. Drive-letter changes require an exclusive native volume lock and attempt rollback if reassignment fails. Partition execution, formatting, physical eject and privilege bypass remain unavailable.

## Validation boundaries

[Milestone 6B](milestone-6b-partition-ui.md) adds typed create/delete previews through the existing planning FFI. Discovery must agree with inventory before regions enter the same explorer/map selection system. Explicit expendable-media designation is tied to persistent identity and the current session. Integer conversion preserves requested bytes; Rust validates geometry and safety. Proposed layouts stay separate from inventory. Generation checks reject late results; selection/input/reset/cancel/refresh invalidate proposals. Refresh while busy invalidates immediately and queues a scan, including the 15-second monitor. Native calls retain the busy lease until completion. No partition execution control exists.

Milestone 6A adds [preview-only partition foundations](milestone-6a-partition-foundations.md). `PartitionPlanningRequest` and `IPartitionPlanningService` use the dedicated `diskvio_partition_plan_json` export through the existing Rust-string SafeHandle. The four original exports and `IDiskService` remain compatible. The WinUI workspace discovers eligibility, selects backend regions/partitions and requests typed creation/deletion previews without dispatching executable actions or deriving policy from visual gaps.

Rust reconciles the existing Storage inventory with read-only native GPT, length, sector alignment and device identity queries. Persistent VPD/serial identity is separate from the versioned layout snapshot. Complete safety evidence and an exact expendable test-media designation are required for plans. Every response reports execution unsupported. Windows hardware, native query permissions and Debug/Release WinUI validation remain pending on this Mac.

Tests link the actual portable models, services and view models without loading WinUI. Native integration tests perform discovery and capability queries only. Mutation tests use fake APIs and command runners; no real storage mutation runs automatically. Existing macOS fixture tests also run on Windows using a test-only plist dependency. Native macOS builds and runtime checks require a Mac. See the milestone report for exact successful commands and outstanding manual checks.
