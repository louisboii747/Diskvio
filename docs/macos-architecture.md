# macOS explorer and operations

Diskvio keeps the Rust workspace and native SwiftUI frontend. Xcode owns the app build and links the Cargo-produced static library through the existing C module map.

## Data ownership and topology

`diskvio-core::DiskInventory` contains physical disks and their physical partitions. APFS containers are normalized separately and referenced by `Partition.apfs_container_id`. A container records every physical-store identifier and owns its APFS volumes. This supports multi-store containers without duplicating them or presenting synthesized APFS volumes as physical partitions. Swift projects the graph into sidebar branches; a shared container can appear beneath each physical store.

Discovery uses `/usr/sbin/diskutil list -plist physical`, `info -plist`, `apfs list -plist`, and the structured `MountedSnapshots` relationships from `list -plist`. IOMedia registry identities come from `/usr/sbin/ioreg -a -r -c IOMedia`. No human-readable discovery output is parsed. Device numbers and partition ordering are reported by macOS; partition-map names identify GPT, MBR, or Apple Partition Map.

Filesystem names preserve platform variants. Partition content types and filesystems are distinct. Missing capacity, mount, UUID, or hardware properties remain optional. Per-device discovery failures produce visible inventory warnings; a failed top-level physical enumeration returns an error. The original `list_disks` API, C entry point, CLI output, and Windows `Get-Disk` discovery remain available. `diskvio-cli --json` prints the richer inventory.

APFS volume allocation and shared container free space are separate measurements. A quota limits reported available capacity; an unlimited volume shares the container ceiling. Container used bytes include metadata and snapshots and need not equal the sum of volume allocations. Physical disk `FreeSpace` is not interpreted as filesystem free space. Mounted snapshots are properties of their source volume with their own identifier, mount point, and UUID; they are neither partitions nor additional APFS volumes.

## FFI boundary

The ABI has three response-producing functions: legacy `diskvio_list_disks_json`, `diskvio_inventory_json`, and `diskvio_operation_json`. Each returns an owned, NUL-terminated UTF-8 JSON allocation or NULL on encoding failure. **Every non-NULL result must be freed exactly once with `diskvio_string_free`.** No pointers to internal Rust collections cross the boundary. Swift copies the response before a `defer` frees it.

Operation input is borrowed UTF-8 JSON with an explicit byte count capped at 16 KiB. The caller must provide a readable buffer for the duration of the call; Rust does not retain or modify it. Invalid JSON, unsupported action names, backend failures, and caught Rust panics produce errors. An invalid pointer or double-free violates the C contract and cannot be repaired by catching a Rust panic.

## Threads and notifications

`DiskStore` is main-actor isolated. Discovery and operations run in detached tasks with Sendable input/output. UI state changes return to the main actor. A per-process Rust mutex serializes management operations across windows.

`DiskMonitor` registers appeared, disappeared, and selected description-change callbacks with a Disk Arbitration session. Its retained C context owns only an AsyncStream continuation. Callbacks and teardown use the main dispatch queue. Teardown unschedules the session, unregisters all callbacks, finishes the stream, releases the context once, and releases the session. Store tasks use weak references and are cancelled when the window disappears.

Known initial appearance notifications do not trigger another scan. Relevant bursts are debounced for 450 ms; there is no timer polling for disks. Watched descriptions cover topology, names, mount paths, UUIDs, and device location rather than continuously changing free space. A scan invalidated by an event is discarded and followed by one coalesced scan. Removal immediately prunes the affected hierarchy and reconciles selection. UUID-based branch identities preserve partition/container/volume selection through refresh and volume renaming.

Apple documents the callback and teardown model in its [Disk Arbitration notification guide](https://developer.apple.com/library/archive/documentation/DriversKernelHardware/Conceptual/DiskArbitrationProgGuide/ArbitrationBasics/ArbitrationBasics.html).

## UI layout and extension points

`ContentView` owns the native navigation shell and the existing `DiskStore`. `DeviceSidebar` uses a native `OutlineGroup` in a selectable sidebar List, with stable node IDs independent of disclosure state. Native outline gutters own indentation; row names receive bounded remaining width and truncate at the tail. At narrow widths, captions fall back to capacity while full type/name/identifier information remains in accessibility labels and help. Selection stays in the store rather than in pane layout state.

`WorkspaceLayout` defines a shared width budget: sidebar 260–480 points (ideal 320), central content at least 480, inspector 240–340 (ideal 280). The content minimum is 1040 × 600 points, with a 1280 × 780 default window. The Devices workspace shows an inline, resizable inspector when its own available width is at least 800 points. Otherwise, the same inspector is available from its toolbar button in a scrollable popover. This decision uses workspace width, so hiding or resizing the sidebar naturally changes the space available. It never changes device selection or triggers discovery.

`DeviceWorkspaceView` contains the current device flow, status reporting and inspector presentation. Other major areas can later supply their own workspace through the navigation shell; unfinished areas are not added to the sidebar today. Settings can use a native Settings scene when actual preferences exist. `DeviceCommands` shares the existing capability-driven rename/mount/unmount/eject controls between the toolbar Actions menu and context menus, keeping the toolbar from accumulating unrelated commands.

`PartitionMapLayout` retains bounded physical geometry helpers and supplies capacity-weighted schematic widths with 48-point minimum targets. The rendered map does not infer gap positions or allocatable space. A synchronized partition list, tooltips and keyboard-focusable buttons expose every tiny partition. Category colours follow backend role/content metadata, and APFS selection highlights its physical store. `PartitionPresentation` centralizes names and category derivation.

Future partition editing can render a separate proposed layout beside the discovered layout. A future operation flow belongs in its relevant workspace: configure parameters, preview affected devices and changes, explicitly confirm any destructive request, then submit to a tested backend and use the current progress/result reporting path. Discovered inventory must remain separate from a proposed edit. Create/delete/resize controls remain unavailable. The confirmed rename flow uses the existing backend; Rust remains responsible for validating every operation.

## Operation policy and permissions

`DiskBackend` separates platform discovery and execution. The macOS implementation accepts mount, unmount, eject, and capability-gated volume renaming. Renaming requires a mounted, explicitly writable external APFS/HFS/FAT/exFAT volume with a verified UUID and confirmed mount snapshot. Swift displays capabilities calculated by Rust; Rust revalidates every request against a fresh inventory. Device identity includes the current IOKit registry entry and available partition/volume UUIDs, so a replacement device reusing a BSD number is rejected. Volume UUIDs resolve targets where available, and duplicate UUIDs are rejected. The JSON operation export additionally supports read-only capability queries and validation; Swift service methods expose these and structured backend errors while preserving existing action menus. Target metadata and registry identities are checked again immediately before invoking diskutil.

All internal disks, unknown device locations, and unidentified devices are inspection-only. APFS volume operations require every physical store to resolve to a verified external disk. Locked volumes and volumes mounted through snapshots have no volume actions. Protected root/system mount paths also block disk ejection. CoreStorage/RAID pools are inspection-only because their complete backing relationships are not modeled. Incomplete discovery blocks operations. Eject is advertised only for external removable or ejectable physical disks.

Each operation requires a toolbar, menu or contextual action and explicit confirmation. Rename additionally requires a read-only validation preview. Pending dialogs are dismissed if their target disappears. Commands use separate arguments with strict BSD identifier validation, an absolute executable, and closed stdin. There is no shell, sudo, force-unmount, authorization helper, formatting, resizing, deleting, raw writing, or image flashing. diskutil's success status or failure diagnostics are shown to the user, followed by a refresh. macOS permission failures and application dissenters remain visible; Diskvio never retries with elevated privileges.

The existing Xcode target uses `ENABLE_APP_SANDBOX = NO` and hardened runtime. This is a directly distributed disk utility deployment model; this change does not disable a previously enabled app sandbox. Inspection needs no administrator privileges. A future sandboxed/App Store distribution requires a separate capability and distribution design; file picker access alone is not a replacement for disk management. See Apple's [Disk Arbitration overview](https://developer.apple.com/library/archive/documentation/DriversKernelHardware/Conceptual/DiskArbitrationProgGuide/Introduction/Introduction.html).

## Limits

Windows has physical-disk, partition and volume inventory with validated mount/unmount, rename and drive-letter operations for supported external USB basic-data volumes. Windows physical eject remains unsupported. See [Milestone 2 backend](milestone-2-backend.md) for the shared JSON API and native verification limits. Other platforms return unsupported errors. Encrypted volumes must be unlocked in Disk Utility; Diskvio does not handle passwords. APFS snapshot management, CoreStorage/RAID topology, and destructive operations are outside these milestones. Partition-map gaps are not plotted or treated as allocatable storage. Diskutil may take up to a minute or longer for OS arbitration; the UI stays responsive and does not offer forced cancellation of an in-flight OS operation.

No software snapshot can make the interval between checking metadata and submitting an OS operation atomic. Device-removal errors are surfaced and stale requests are rejected; management commands are never retried automatically.

See [macOS Milestone 5](milestone-5-macos-refinement.md) for additive metadata, label policy, app icons and the latest validation boundaries.
