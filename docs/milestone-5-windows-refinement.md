# Milestone 5 — Windows refinement and management foundations

Date: 10 October 2026.

## Architecture reviewed and preserved

The Windows app remains C# / .NET 10 / WinUI 3, packaged through the existing single-project MSIX pipeline. `Diskvio.slnx` builds the app, its UI-independent xUnit project and the Rust x64 DLL using `windows/build/Diskvio.Rust.targets`.

The data path is unchanged:

`DevicesPage → StorageWorkspaceViewModel → IDiskService / DiskService → NativeMethods → diskvio-ffi → diskvio-core → Windows backend`.

`diskvio_inventory_json` supplies physical disks, partitions and separately identified volumes. `diskvio_operation_json` supports capability queries, validation and execution. Rust owns each returned UTF-8 JSON allocation; the existing managed safe handle frees it exactly once. Existing C ABI exports, ownership rules, legacy discovery and status/error envelopes are preserved.

Windows discovery continues to use the **existing Rust-owned** Windows Storage PowerShell script (`Get-Disk`, `Get-Partition`, `Get-Volume`). The frontend runs no PowerShell or DiskPart. Mutations use native Windows volume APIs in Rust, never interpolated shell commands. Policy, stable identity, parameter validation, final rediscovery and structured errors remain in `diskvio-core`.

The page, workspace view model, storage nodes and partition-map control have been extended in place. `PartitionPresentation` is a small UI-independent naming helper, not another discovery or authorization layer.

## Additive backend / JSON contract

| Object | Field | Meaning |
| --- | --- | --- |
| Partition | `role` | Windows classification: `efi_system`, `recovery`, `microsoft_reserved`, `basic_data`, `unknown`. Optional for older / macOS inventories. |
| Device | `health_status` | Windows-reported health string, omitted from presentation when unavailable. |
| Device | `operational_status` | Windows-reported status strings; defaults to an empty list. |
| Device safety | `page_file` | Whether a reported partition access path contains an active Windows paging file. Unknown blocks Windows volume management. |
| Capabilities | `unsupported_reason` | Backend explanation when the object has no supported actions. |
| Capabilities | `label_max_length` | Filesystem-specific rename limit for authorized volumes. |
| Capabilities | `limitations` | Explicit platform limitations, including unsupported partition editing and physical eject. |
| Request | `volume_label` | New label for `rename_volume`; an explicit empty string removes a label. |
| Request | `drive_letter` | Uppercase D–Z target for `set_drive_letter`. |
| Request | `expected_mount_points` | Access-path snapshot confirmed by the user. Required to match current metadata for both new actions. |

New action values are `rename_volume` and `set_drive_letter`. The request fields default to absent, preserving existing mount/unmount/eject callers. Irrelevant operation parameters are rejected. Create/delete/format action values are not accepted by the FFI decoder.

Paging-file discovery uses `Win32_PageFileUsage` inside the Rust-owned inventory script. If that discovery fails, a warning is surfaced and all operations are withheld. Missing active/shadow-copy flags no longer silently become safe values. New fields default safely when decoding older JSON; unknown or missing safety metadata does not authorize the new actions.

No explicit allocatable-region contract was added. Complete, non-overlapping offsets permit drawing **unmapped space / metadata**, but this is not proof that those bytes can be allocated.

## Naming and metadata

Partition primary names follow this order:

1. First nonempty associated volume label.
2. Friendly role name, determined from known GPT type GUIDs, supported MBR type codes or recognized Windows partition-type names.
3. `Partition X`, or `Partition` when the number was not reported.

Examples include EFI System Partition, Recovery Partition, Microsoft Reserved and Basic Data. Rust emits the friendly partition name; the Windows presentation helper also supports older contracts with known type GUIDs. An unlabeled volume uses its drive letter or “Unlabelled volume”. Raw volume GUID paths and `PhysicalDriveNPartitionM` remain technical identifiers in tooltips, accessibility descriptions and the inspector.

The partition table presents filesystem, mount location, capacity and role. Volume rows are indented under their partition. The selected-item inspector adds volume label, partition number/type, exact byte capacity, reported usage, health/status, hardware connection, safety flags, identifiers and supported actions. Partition health/filesystem summaries use associated volume metadata without inventing a partition filesystem.

System, Boot, Recovery, Paging file, Read-only, Offline, Hidden, External, Removable and USB tags appear only from reported metadata or known partition roles. A USB connection alone does not claim removability. Missing filesystem/capacity metadata stays explicitly unreported; it is not converted to zero or presumed unformatted.

## UI and partition layout

- Native command bar with refresh, rename label, drive letter, mount and unmount.
- Compact physical-disk navigation, a partition/volume table and a selected-item inspector.
- Inspector sits alongside content on wide workspaces and moves below it at narrower widths. Compact workspaces move navigation above the detail area.
- Right-click and keyboard context menus select the relevant object and query its current capabilities before presenting actions. They include identifier copy and refresh.
- A partition with exactly one volume can address that volume through the same actions; multi-volume partitions require an explicit volume selection.
- Selection synchronizes the physical bar and table. Selecting a volume highlights its containing partition.
- Reported capacity proportions are preserved for normal segments. Tiny partitions receive a 36-DIP minimum allocation in the visual map; unmapped gaps receive 4 DIP. Dense maps scroll horizontally. The interface explicitly discloses this visual adjustment.
- Tooltips retain full names, types, capacity, offsets and technical identity. Native button focus/activation and automation names make small segments inspectable without relying on their visible text.
- Primary labels truncate inside bounded cells; full text remains available through tooltip/inspector.
- Refresh, loading, empty, partial-discovery, success and structured-error states remain visible. All mutation controls are disabled during confirmation/execution.

### Colour meanings

| Category | Treatment |
| --- | --- |
| EFI / system / boot | Warm amber |
| Recovery | Green |
| Microsoft reserved | Neutral grey |
| Basic data | Blue |
| Unknown type | Dark / medium neutral |
| Unmapped space / metadata | Very dark neutral |
| Selected partition | Native accent border |

Light and dark theme dictionaries provide corresponding hues; text and control surfaces keep Fluent semantic resources. High contrast uses system colours, with role text, tooltips, table rows and selection/focus supplying meaning independently of colour. No motion was added. Runtime theme, high-contrast and assistive-technology checks remain part of the manual procedure below.

## Supported management operations

| Operation | Scope and execution |
| --- | --- |
| Refresh / rescan inventory | Read-only backend rediscovery; Ctrl+R, F5, context menu and existing 15-second monitor. No disk initialization or physical hardware rescan command is issued. |
| Rename volume label | Backend-approved external USB basic-data NTFS/FAT/exFAT volume; native `SetVolumeLabelW` against the verified volume GUID. |
| Assign / change drive letter | Same supported volumes, with zero or one drive-letter mount; native lock, dismount, remove old letter if present, and `SetVolumeMountPointW`. Requested occupied letters are rejected before removal. |
| Mount / unmount | Existing gated external USB volume support remains; both use explicit confirmation and fresh backend validation. |

Label limits are 32 UTF-16 code units for NTFS and a conservative 11 for FAT/exFAT. Control characters, surrounding whitespace and the documented conservative punctuation set (`\ / : * ? " < > | + . , ; = [ ]`) are rejected. Empty label removal is explicit in the preview. D–Z is the supported drive-letter range; the dropdown does not claim that every shown letter is free.

Each new operation follows: select target → query capabilities → edit and preview exact change → explicit Apply (Cancel is the default) → query current capabilities → validate request in Rust → rediscover/revalidate during execution → check native access paths → mutate → report outcome/error → refresh and restore the selection where its identity remains available.

New management requests also validate the confirmed mount-path snapshot. Device disappearance, replaced hardware, changed paths, ambiguous identity or incomplete discovery blocks execution. Internal/unknown-location disks, system/boot/EFI/recovery/reserved partitions, paging-file volumes, hidden/shadow-copy/read-only/offline volumes, unsupported filesystems and directory/multiple mounts are inspection-only.

Drive-letter changes require an exclusive volume lock; open files cause a safe failure. Diskvio also refuses to change a volume containing its own executable or working directory. If assigning the new letter fails after removing the old one, Rust attempts to restore the original letter. A failed restoration is reported explicitly; it is not hidden as success. There is no forced reassignment, implicit elevation or automatic retry.

Native API references: [SetVolumeLabelW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setvolumelabelw), [drive-letter assignment](https://learn.microsoft.com/en-us/windows/win32/fileio/assigning-a-drive-letter-to-a-volume), [SetVolumeMountPointW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setvolumemountpointw). Windows permits only one drive letter per volume, so reassignment removes the old letter before attempting the new one, with explicit rollback handling.

## Intentionally unsupported

Create/delete/resize/move partitions, formatting, wiping, ISO/image writing, system-disk editing, physical eject, directory-mount editing, dynamic/pooled storage editing and automatic privilege escalation are unavailable. No destructive command is exposed for UI completeness.

Create/delete are deferred because the current backend does not expose authorized, aligned allocatable regions or a safe partition mutation implementation. The gap renderer is not an allocation API. A later milestone must establish that contract and its safety policy before exposing either action.

macOS retains its existing mount/unmount/eject policy and SwiftUI frontend. New Windows-only actions are never advertised for macOS targets and are explicitly rejected by its execution backend. Additive JSON properties are ignored by existing Swift decoding. The existing macOS fixture tests now also compile/run on Windows through a test-only `plist` dependency; no Windows production dependency or diskutil execution is added.

## Validation results

Validation was non-destructive. No real label, drive-letter, mount/unmount or partition operation was executed. Native mutations were exercised only through deterministic fake APIs / command runners.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --workspace --locked` | Passed. |
| `cargo test --workspace` | 49 passed: 43 core (including 10 existing macOS fixture tests) and 6 FFI tests. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed without warnings. |
| C# deterministic tests, warnings as errors | 47 passed. |
| C# read-only native integration tests | 2 passed; real disk discovery and capability queries only. |
| Windows Debug x64 solution, warnings as errors | Passed without compiler or nullable warnings. |
| Windows Release x64 solution, warnings as errors | Passed without compiler or nullable warnings. |
| Packaged launch | App launched through the existing `dotnet run` packaging path; a responding Diskvio process was observed. This is startup evidence only. |
| `git diff --check` | Passed. |

New tests cover friendly naming and legacy GUID classification, unknown metadata, tiny and dense maps, protected/unknown/partial states, exact capability identity, single-volume action resolution, cancellation, parameter forwarding, failed/disappearing targets, native locking, occupied letters, changed paths, successful assignment, rollback and rollback failure. Existing stale-identity, unsupported filesystem, mount/unmount and topology tests continue to pass.

**Blocked validation:** the Computer Use Node helper could not initialize because its sandbox launcher encountered the unrelated unformatted `G:` volume (`os error 1005`). Resetting and retrying failed. Rendered light/dark/compact/high-contrast inspection, keyboard/screen-reader interaction and live USB mutations have not been certified. Native macOS/Xcode build and runtime validation also require a Mac. These limitations do not imply that an actual USB mutation was attempted.

## Safe manual verification using an expendable external USB drive

1. Use a backed-up, expendable USB drive with a known basic GPT or MBR data partition and NTFS/FAT/exFAT. Run Diskvio from another disk. Check the disk number, capacity, connection and technical identifier before choosing any action.
2. First inspect only: compare labels, filesystem, mount location, capacities, types and reported status with Windows Disk Management. EFI/recovery/reserved/system disks must stay inspection-only. Check an unformatted or unsupported device without accepting any Windows format prompt.
3. Resize the window across compact, normal and wide layouts. Check light/dark appearance and Windows high contrast manually. Tab through tiny map segments, activate one with Enter/Space and confirm table/inspector synchronization. Use Shift+F10 and right-click on both disks and partitions/volumes. Verify full tooltips and screen-reader names.
4. Refresh repeatedly, then safely disconnect/reconnect an idle USB device. Confirm removed objects disappear, stale commands disable, and re-selection follows identity rather than an accidentally reused disk number.
5. On the expendable supported volume, open Rename label, inspect its target/preview and **Cancel first**. Then deliberately apply a new temporary label. Verify the result in Diskvio and Windows, refresh and restore the original label. Try an overlong/invalid label and confirm rejection. An empty label removes the label only after explicit Apply.
6. Record the original drive letter. Close every file and app using the USB volume. Select an unused D–Z letter, review the preview and **Cancel first**. Then explicitly apply and verify the new letter in Windows and Diskvio; restore the original letter the same way. Existing shortcuts may need adjustment.
7. Try an already occupied letter; it must fail without removing the original letter. Open a file on the USB volume and try a letter change or unmount; failure to lock must leave the old assignment intact. Do not use a drive with active paging files or any system role.
8. To check hot-unplug handling, disconnect an **idle** expendable USB device while its confirmation dialog is open, then try Apply: the operation must fail validation and refresh the device away. Do not unplug while a mutation is executing.
9. If Windows returns access denied, confirm that the error is shown with a refreshed state. Do not bypass Diskvio policy or trigger automatic elevation. Test any administrator launch deliberately and only against the expendable drive.
10. Do not test create/delete/format/wipe/image writing; those actions must remain absent. Rollback-failure injection belongs to the fake-API tests, not a real drive.

## Recommended Milestone 6

- Complete the blocked native visual/accessibility checks and deliberate USB mutation verification on real hardware.
- Add authoritative allocatable-region objects with reserved metadata/alignment bounds, stable disk/region identity and backend operation-specific reasons.
- Define and test an external-only create/delete safety policy, including disk/partition protection, paging/boot/BitLocker/storage-pool exclusions and TOCTOU handling, before adding any native partition mutation API.
- Provide operation-specific preflight previews, permission diagnostics and a controlled privilege strategy. Retain cancellation by default, explicit confirmation, structured results and post-action rediscovery.
- Add Windows UI automation on a runner with a working desktop helper and native macOS CI coverage. Keep real storage mutations out of unattended tests.

No commit or push was performed.
