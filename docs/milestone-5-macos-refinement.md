# Milestone 5 — macOS refinement and volume management

Implemented 10 October 2026. Changes extend the existing SwiftUI app and Rust backend. No real storage mutation was performed during implementation or testing.

## Architecture

The existing path remains SwiftUI → DiskStore → DiskService → C ABI JSON → diskvio-ffi → diskvio-core → macOS diskutil/ioreg. Rust owns discovery, target identity, capabilities, validation and execution. Swift never discovers storage through shell scripts or invokes diskutil itself. FFI allocation and string-free ownership are unchanged.

Physical disks own physical partitions; normalized APFS containers reference their physical stores and own volumes. Shared APFS volume capacity is never drawn as additional physical partitions. Disk Arbitration monitoring, coalesced refreshes and the resizable inspector are retained.

`PartitionPresentation` centralizes friendly names and categories. `PartitionMapLayout.visualWidths` supplies independently testable schematic geometry. `DeviceNode.operationTarget` centralizes frontend action gating; fresh backend checks remain authoritative. The small Swift package in `macos/Package.swift` tests the actual app models on Windows or macOS, independently of SwiftUI and hardware. Xcode remains the native application build system.

## Contract changes

All additions are optional/defaulted; legacy discovery and operation entry points remain compatible.

| Field | macOS source and meaning |
| --- | --- |
| `device.volume_label` | `VolumeName` / APFS volume `Name`, separate from media names and synthesized identifiers. Windows consumers may ignore it. |
| `partition.role` | Existing shared categories populated from exact content types: EFI, recovery, data or unknown. |
| `partition.number` | Physical slice number parsed from macOS's reported BSD identifier. |
| `device.mount_points` | macOS mount path as a zero-or-one-element array, for confirmed rename snapshots. `mount_point` remains available. |
| `device.safety.read_only` | True if either `ReadOnlyMedia` or `ReadOnlyVolume` is true; false only if both are explicitly false; otherwise unknown. |
| `device.safety.system/boot/recovery` | Known EFI/recovery content and explicit APFS System/Preboot/Recovery roles. These do not assert that a disk is the currently booted disk. |
| `device.health_status` | Reported SMART status, when available; never synthesized. |
| `device.stable_id` | Existing IOMedia identity is decoded by Swift for stable hierarchy selection. |
| capabilities | Swift now decodes `unsupported_reason`, `limitations`, and `label_max_length`. |
| rename requests | `action: "rename_volume"`, `volume_label`, `expected_identity`, and `expected_mount_points`; validation queries forward the same parameters. |

## Naming and UI

Names prefer the explicit filesystem label, then a recognized partition role/type, then `Partition N`. Older JSON can retain a meaningful filesystem label in `name`. Physical-store names include **APFS Physical Store**, **Apple HFS Partition**, **EFI System Partition**, and **Recovery Partition**. Technical identifiers, backend names, offsets and stable IDs remain secondary inspector metadata. Containers use **APFS Container** with their identifier in secondary information.

The partition map uses consistent semantic categories: orange for system/EFI, green for recovery, grey for reserved/metadata, blue for data/APFS stores, and the native secondary-label tone for unknown types. Names and a legend accompany colours. Selected partitions receive an accent border, and selecting an APFS volume highlights its physical store on the current disk.

The 64-point bar gives every partition a minimum 48-point target, with remaining width weighted by reported capacity and 4-point spacing. Dense layouts scroll horizontally. Exact capacities, formats and mount paths remain readable in a synchronized list with tooltips, accessibility labels, keyboard-focusable buttons and context menus. Widths are explicitly described as schematic. Gaps and unallocated regions are not plotted or offered as creation targets; partition-table gaps are not proof of usable space. Unknown capacities remain unknown.

The selected-item area adds restrained metadata badges, direct contextual actions and backend-provided inspection-only explanations. The inspector adds label, role, number, offset, SMART/status, protection and advanced identity fields. System colours and native controls retain light/dark appearance support; runtime appearance and VoiceOver checks still require a Mac.

## Supported operations and safety

- **Rename volume:** mounted external APFS, HFS, FAT or exFAT filesystems with a verified volume UUID, explicitly writable media and volume, resolved external backing stores, and complete discovery. Internal disks, protected roles/paths, EFI/recovery disks, locked volumes, mounted snapshots, ambiguous identities, unsupported filesystems and unresolved pools are blocked.
- **Mount/unmount:** existing backend-authorized external volume support is retained. Mount now also requires explicit confirmation.
- **Eject:** existing support for verified external removable/ejectable physical disks is retained, including parent-disk resolution from a volume.
- **Refresh:** manual Command-R and existing Disk Arbitration events; selection survives label changes and BSD-number changes where stable identity is available.

Rename uses an editable sheet → read-only backend preview → explicit confirmation. Escape/Cancel dismisses it; Rename has no default Return shortcut. The preview shows the existing target, old/new labels and mount path. macOS can change a path under `/Volumes`, affecting shortcuts and apps using that path. Execution repeats capability and identity validation, rescans topology, checks current UUID/location/mount state and IOMedia registry identity, then rechecks writability immediately before `diskutil renameVolume <verified-UUID> <label>`. Arguments are separate, never shell-interpolated. No force, sudo, retry or elevation helper is introduced.

The backend deliberately limits APFS/HFS names to 63 UTF-16 units and all macOS names to 127 UTF-8 bytes. FAT/exFAT retain the conservative 11-unit policy. Empty names, surrounding whitespace, control characters, leading dots/hyphens and filesystem-inappropriate characters are rejected. These are Diskvio's conservative policy limits, not claims about maximum filesystem capacities.

Success and structured failures reach the existing notice area, followed by refresh. Pending dialogs close if their target disappears or is replaced. Native OS arbitration is not atomic with software checks; hot-removal failures remain possible and are surfaced without retry.

**Intentionally unsupported:** drive letters on macOS; create/delete/resize partitions or APFS volumes; formatting, wiping, raw writes, ISO flashing, unlocking, snapshot management and CoreStorage/RAID editing. No controls imply these are available.

Apple documents [shared APFS capacity and system volume groups](https://support.apple.com/guide/disk-utility/add-delete-or-erase-apfs-volumes-dskua9e6a110/mac), [diskutil's role as the command-line counterpart of Disk Utility](https://developer.apple.com/library/archive/documentation/OpenSource/Conceptual/ShellScripting/ForMoreInformation/ForMoreInformation.html), and [disk naming restrictions](https://support.apple.com/guide/mac-help/rename-files-folders-and-disks-on-mac-mchlp1144/mac). Diskvio applies stricter operation policy than the general OS tools.

## App icons

`assets/Diskvio Icon.png` supplies all ten macOS AppIcon catalogue entries (16–1024 pixels) and the Windows PNG tile/store/splash variants. `assets/diskvio-icon-windows.ico` is copied unchanged to the executable's existing icon path. Existing Xcode and MSIX references already select those assets, so no alternate icon-loading architecture was added.

`python assets/export_icons.py` reproduces the exports with Pillow. It only resizes/centres the provided artwork and preserves the source files. All macOS slot dimensions and image modes were checked; Windows PNG decoding and byte-for-byte ICO equality passed. Windows Debug and Release builds included the assets. Native macOS Dock/Finder rendering and Windows shell icon-cache behaviour have not been visually verified.

## Validation results

Run on this Windows host:

| Check | Result |
| --- | --- |
| Rust formatting / workspace check / Clippy all targets with `-D warnings` | Passed |
| `cargo test --workspace --locked` | 55 passed: 49 core + 6 FFI; macOS commands use fixtures only |
| Portable Swift package, Swift 6.3.3, compiler warnings as errors | 12 tests passed, including seven parameterized type/category cases |
| All native app/test Swift files parsed | Passed; parsing is not SDK type checking |
| C# deterministic tests, warnings as errors | 47 passed |
| C# read-only native inventory/capability tests | 2 passed |
| Windows solution Debug / Release x64, warnings as errors | Both passed |
| Icon slots and file integrity / diff whitespace | Passed |

Rust coverage includes rename command arguments and UUID targeting, missing writability, final read-only changes, stale mount snapshots, naming/protection, filesystem-specific label validation, protected APFS roles, locked volumes, mounted snapshots and unresolved stores. Portable Swift tests cover naming, classification, exact request encoding, capability compatibility, tiny/unknown geometry, badges, selection and action gating. Three additional native DiskStore tests cover confirmation, rename parameters, and removal while dialogs are pending.

**Blocked validation:** this host has no Xcode, macOS SDK, SwiftUI runtime or Apple hardware. The native app build, native DiskStore test execution, asset-catalog compilation, actual rename integration and final light/dark/VoiceOver checks remain unverified. The installed Windows Swift tools initially selected mismatched 6.3/6.4 components; tests passed after selecting a matched 6.3.3 compiler, SDK and runtime with SwiftPM's native build system. No global toolchain configuration was changed.

## Safe manual verification on a Mac

1. Build/test the existing Diskvio Xcode scheme on My Mac; also run `swift test --package-path macos`. Install the matching Rust Apple target first. Confirm Dock/Finder/About icons use the supplied artwork.
2. Inspect the internal disk. Confirm names, APFS roles, shared capacity, badges, inspector and disabled management. Check light/dark appearance, narrow inspector popover, keyboard navigation and VoiceOver.
3. Connect an **expendable external USB drive** containing a non-system APFS/HFS/exFAT/FAT volume. Confirm labels, formats and UUIDs against Disk Utility. EFI/system/recovery-bearing disks should remain protected.
4. Inspect tiny partitions using the bar, list, keyboard and context menus. Select an APFS volume and confirm the physical-store highlight without an extra physical segment.
5. Open Rename; preview a new label, cancel, and verify the label is unchanged. Try an invalid name and confirm the backend error. Then explicitly confirm a valid rename on the expendable volume; verify the result in Finder/Disk Utility and selection after refresh.
6. Close files, then exercise confirmed unmount/mount/eject on that drive. Check a busy/permission failure is reported without force or elevation.
7. While a preview is open, safely eject/remove the test drive. Confirm pending UI closes and no operation targets a replacement device. Reconnect and refresh. Never unplug while a mutation is executing.

## Recommended Milestone 6

Add macOS CI for the native scheme and portable tests, verify real external-volume rename on APFS/HFS/exFAT/FAT, and complete native appearance/accessibility coverage. Introduce an explicit backend allocation-region contract and operation plans before considering partition creation/deletion; model APFS volume management separately from physical partition edits. Continue to keep internal/system targets inspection-only.
