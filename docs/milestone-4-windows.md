# Milestone 4 — Native Windows integration

Implemented on Windows 11, 10 October 2026. This extends the existing WinUI app, solution, typed bindings and tests; it does not scaffold another frontend or alter the shared backend.

## Architecture

The existing `windows/Diskvio.slnx` supports Visual Studio 2026, Debug/Release and x64. The app targets .NET 10 and Windows App SDK using its existing package versions and CommunityToolkit.Mvvm dependency. No dependencies were added.

`MainWindow` retains its native title bar, NavigationView and conditional Mica. `DevicesPage` owns controls, the confirmation dialog and a 15-second read-only refresh timer. `StorageWorkspaceViewModel` owns asynchronous inventory, selected disk/device, inspector, capability gating, operation sequencing and busy state. `StorageNode` is a presentation record over existing storage contracts. The legacy physical-disk view model remains compatible with its existing tests.

All device discovery and operations use the existing `IDiskService` / `DiskService`. Service calls run on worker threads; view-model updates resume on WinUI's synchronization context. The portable `PartitionLayout` calculates conservative geometry; `PartitionMap` renders native buttons and uses the same partition identity as the explorer and inspector.

## Files created

- `windows/Diskvio.Windows/ViewModels/StorageWorkspaceViewModel.cs`: inventory, selection, inspector, capability queries and safe action workflow.
- `windows/Diskvio.Windows/ViewModels/StorageNode.cs`: device presentation and concise accessibility names.
- `windows/Diskvio.Windows/Models/PartitionLayout.cs`: validated physical partition geometry and explicit incomplete-data fallback.
- `windows/Diskvio.Windows/Controls/PartitionMap.cs`: proportional segments, selection, tooltips and accessible labels.
- `windows/Diskvio.Windows/Services/AppDiagnostics.cs`: best-effort application/startup exception logging under local application data, without swallowing the original app failure.
- `windows/Diskvio.Windows.Tests/WorkspaceTests.cs`: deterministic geometry, selection, capability, identity, confirmation, busy-state and failure tests using a test-only service.
- `windows/Diskvio.Windows.Tests/NativeInventoryTests.cs`: repeated real inventory and capability queries; never executes an operation.
- `docs/milestone-4-windows.md`: this report.

## Files modified

- `windows/Diskvio.Windows/Views/DevicesPage.xaml` and `.xaml.cs`: native explorer, commands, inspector, compact layout, confirmations and monitoring.
- `windows/Diskvio.Windows/MainWindow.xaml`: shell footer now describes the storage explorer.
- `windows/Diskvio.Windows/App.xaml.cs`: startup and unhandled-error diagnostics.
- `README.md` and `docs/windows-architecture.md`: current capabilities and architecture.

The solution, project dependencies, Rust build targets, C# FFI bindings, existing tests, Rust crates, macOS source and shared ABI were reused without modification.

## Windows UI features

- Actual physical-disk sidebar with capacity, disk number, connection and device icons.
- Native Refresh, Mount, Unmount and visibly unavailable Eject controls. Refresh supports Ctrl+R and F5.
- Partition and volume explorer; selecting either updates the inspector and associated partition highlight.
- Inspector with backend-reported identifiers, exact/decimal capacity, used/available bytes, filesystem, drive letter/mount path, UUIDs, partition types, offset, connection and safety flags. Missing optional metadata is omitted; unknown capacity is explicitly marked.
- A proportional physical partition map only when disk capacity, every partition size/offset and non-overlapping bounds are known. Neutral regions are labeled **unallocated / partition metadata**, because gaps may include reserved metadata. Missing, overlapping, overflowing or out-of-bounds geometry never invents positioning or gaps; known partition capacities remain available in an explicitly labeled fallback and legend.
- Full-size partition legend targets for tiny physical segments. Tooltips, accessible names, native focus/selection controls and selectable inspector text.
- Fluent semantic theme resources and inherited system light/dark theme. No custom animation or added visual framework.
- Independent scrolling, side-by-side workspace at wider widths and stacked navigation/details below 850 DIPs.
- Distinct loading, empty, backend warning, error and successful-operation messages. Unattached volumes remain inspectable without pretending they belong to a disk.
- Polling refresh on the loaded page, stopped on unload. Stable selection uses device identifiers and available stable identity/token; missing or changed devices fall back safely. Failed discovery discards stale devices.

## Rust integration and safety

Existing exports are retained:

| Export | Usage |
| --- | --- |
| `diskvio_inventory_json` | Physical disks, partitions, volumes and partial-discovery warnings |
| `diskvio_operation_json` | Supported-operations query, validation and explicit supported execution |
| `diskvio_list_disks_json` | Existing legacy API/tests |
| `diskvio_string_free` | Release each Rust result exactly once |

The existing resolver loads the absolute app-local `diskvio_ffi.dll`, requires x64 and checks all four exports. SafeHandle owns each Rust allocation; the service copies and strictly decodes UTF-8, deserializes typed models, then disposes the handle on success and error paths. Request bytes use Cdecl and pointer-sized lengths. Structured errors retain backend codes and native error numbers.

The existing MSBuild target runs locked Cargo for `x86_64-pc-windows-msvc` and copies the correct Debug/Release DLL into build, publish and MSIX output. Rust remains authoritative: the frontend does not run PowerShell/DiskPart or independently implement volume management.

For a Mount/Unmount action:

1. Require a selected external USB volume, inventory identity/action and matching queried backend capability. Partial-discovery warnings disable actions.
2. Acquire the shared UI busy state, disabling duplicate actions, refresh and selection throughout the confirmation and native call.
3. Show a confirmation with Cancel as the default. Unmount asks users to close files and applications using the volume.
4. Query capabilities again and require the same identifier, identity and requested action.
5. Request backend validation and check that its response matches the request.
6. Execute through Rust, which independently revalidates identity and safety; verify the returned action/identifier.
7. Display the result and refresh. Failure also attempts refresh to detect a disconnect or changed state. Capability responses from an older selection are discarded.

No format, repartition, resize, wipe, image-writing, privilege bypass or automatic operation exists. Windows physical eject is unsupported and disabled. Internal/system/protected/unsupported devices remain inspection-only. No real mount/unmount was executed during this milestone's validation.

## Validation

Successful checks in this environment:

- `cargo test --workspace --locked`: **31 tests passed** (25 core, 6 FFI; CLI/doc targets also successful). Existing platform operation tests use test doubles rather than real disk operations.
- `dotnet test windows/Diskvio.Windows.Tests -p:DiskvioPortableTests=true --filter 'Category!=NativeIntegration' --nologo`: **33 passed**.
- `dotnet test windows/Diskvio.Windows.Tests -p:Platform=x64 --nologo`: **35 passed**, including two read-only native integration tests. The richer integration test discovers actual topology twice and queries physical-disk/volume capabilities through the current DLL.
- Visual Studio 2026 MSBuild of `windows/Diskvio.slnx`, Debug and Release, x64, with `/warnaserror`: successful. Nullable checking remains enabled. No compiler warnings reported.
- Packaged launch using `dotnet run --project windows/Diskvio.Windows -p:Platform=x64 --no-build --no-launch-profile`: actual WinUI window opened and remained running.
- Read-only Windows UI Automation inspection: the running window displayed **five actual physical disks**, physical partition segments, explorer rows and inspector metadata. Selecting the Kingston USB disk and then its volume displayed **exFAT**, drive **U**, mount location **U:\**, exact capacity and available space. No operation button was invoked.
- The attached USB volume's current backend capability query returned no actions; Mount/Unmount remained disabled, correctly preserving the backend policy. Eject remained disabled. Positive operation flow is covered by test-only services; this is not hardware execution proof.
- Native startup validation found and fixed an invalid `Icon="Eject"` shorthand: Eject is absent from the Symbol enum. The UI now uses the documented Fluent font glyph. This failure was caught by launching, despite a successful XAML compile. See [Symbol enum](https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.xaml.controls.symbol) and [Fluent glyph reference](https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font).

The managed/WinUI build needed access to installed SDKs and NuGet paths beyond the restricted shell sandbox. Those non-destructive build/test requests were approved and completed. No toolchain reinstall or package-version change was needed.

## Limitations and outstanding checks

- Real USB mount/unmount, busy-file rejection, unplug during execution and positive enabled hardware-action states remain manual verification. They were deliberately not exercised automatically. Backend-rejected devices must not be enabled to make a demonstration work.
- The five-disk runtime check verified control/data presence and selection through accessibility APIs. A complete visual review across dark/light/high-contrast themes, large text, DPI settings, compact sizes and Narrator remains outstanding. Native controls and semantic resources provide the implementation, not proof of every accessibility condition.
- Device changes are detected by a 15-second polling refresh rather than Windows device notifications. Discovery/capability calls may take several seconds; the UI stays asynchronous. Native discovery supplies its existing timeout; managed cancellation of an already-running native call is not introduced.
- Only Windows x64 is supported. The project retains its minimum Windows 10 build declaration, but runtime validation here used Windows 11.
- The app uses local development package identity. Store/release signing and distributable installer validation are outside this milestone.
- macOS source, headers, JSON contracts and Rust code are unchanged, preserving their interfaces. Xcode/SwiftUI builds and macOS-only tests could not run on Windows; Windows Rust tests do not certify Apple-specific compilation.

## Safe manual USB verification

1. Build Debug | x64 and launch through Visual Studio's packaged profile or the documented CLI command. Keep internal/system devices inspection-only throughout.
2. Connect a noncritical external USB volume with backed-up data. Confirm its model, capacity, identifier, filesystem and drive letter in Diskvio and Windows. Never identify the target using a drive letter alone.
3. Select its volume. If the backend reports no supported action, inspect the metadata and stop operation testing; do not modify flags or bypass policy. Confirm physical Eject stays unavailable.
4. First open an available action's confirmation and cancel. Verify no mount state changes and command availability recovers.
5. For an explicitly permitted **Unmount**, close all files/apps on that USB volume, confirm the exact target and submit once. Verify progress, success/error, automatic refresh and stable selection. A busy or permission error is an error, never a simulated success.
6. If the backend subsequently permits **Mount**, confirm once and verify the reported mount location after refresh. Do not format or repartition if mounting is unsupported.
7. Disconnect/reconnect the USB during idle inspection. Verify refreshed inventory removes/restores it and never directs an old selection to another disk. Exercise disconnect/identity errors only with this external test USB and no pending writes.
8. Check keyboard selection, Ctrl+R/F5, map/legend/explorer selection consistency, small windows, system themes, high contrast, text scaling and Narrator.

## Suggested Milestone 5

Add event-driven Windows device monitoring with refresh coalescing, complete the hardware/accessibility/DPI verification matrix, and validate signed packaging/update distribution. Investigate missing Windows safety metadata using read-only backend evidence where supported USB hardware reports no actions. Any future physical-eject implementation should be a separately scoped backend capability with identity/safety tests before enabling a UI command. Destructive operations remain outside this proposal.
