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

## Operation policy and permissions

`DiskBackend` separates platform discovery and execution. The macOS implementation accepts only mount, unmount, and eject. Swift displays capabilities calculated by Rust; Rust revalidates every request against a fresh inventory. Device identity includes the current IOKit registry entry and available partition/volume UUIDs, so a replacement device reusing a BSD number is rejected. Target metadata and registry identities are checked again immediately before invoking diskutil.

All internal disks, unknown device locations, and unidentified devices are inspection-only. APFS volume operations require every physical store to resolve to a verified external disk. Locked volumes and volumes mounted through snapshots have no volume actions. Protected root/system mount paths also block disk ejection. CoreStorage/RAID pools are inspection-only because their complete backing relationships are not modeled. Incomplete discovery blocks operations. Eject is advertised only for external removable or ejectable physical disks.

Each operation requires a toolbar or context-menu action. Commands use separate arguments with strict BSD identifier validation, an absolute executable, and closed stdin. There is no shell, sudo, force-unmount, authorization helper, formatting, resizing, deleting, raw writing, or image flashing. diskutil's success status or failure diagnostics are shown to the user, followed by a refresh. macOS permission failures and application dissenters remain visible; Diskvio never retries with elevated privileges.

The existing Xcode target uses `ENABLE_APP_SANDBOX = NO` and hardened runtime. This is a directly distributed disk utility deployment model; this change does not disable a previously enabled app sandbox. Inspection needs no administrator privileges. A future sandboxed/App Store distribution requires a separate capability and distribution design; file picker access alone is not a replacement for disk management. See Apple's [Disk Arbitration overview](https://developer.apple.com/library/archive/documentation/DriversKernelHardware/Conceptual/DiskArbitrationProgGuide/Introduction/Introduction.html).

## Limits

Windows retains basic physical discovery; rich partition exploration and operations are macOS-only. Other platforms return unsupported errors. Encrypted volumes must be unlocked in Disk Utility; Diskvio does not handle passwords. APFS snapshot management, CoreStorage/RAID topology, and destructive operations are outside these milestones. Partition-map gaps are shown without asserting that all gaps are allocatable storage. Diskutil may take up to a minute or longer for OS arbitration; the UI stays responsive and does not offer forced cancellation of an in-flight OS operation.

No software snapshot can make the interval between checking metadata and submitting an OS operation atomic. Device-removal errors are surfaced and stale requests are rejected; management commands are never retried automatically.
