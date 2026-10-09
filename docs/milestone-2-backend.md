# Disk management essentials

Milestone 2 adds shared volume-operation policy, Windows partition and volume discovery, structured operation errors and frontend service bindings. Existing C exports and the legacy physical-disk response remain compatible. No format, erase, partition editing, resizing, raw writing or image-writing operation exists.

## Inventory

`diskvio_inventory_json()` returns `{"status":"ok","inventory":{...}}`. Existing `disks`, `apfs_containers` and `warnings` fields retain their meaning. The additions are optional when decoding older snapshots:

- `Device.stable_id`, `drive_letter`, `mount_points` and `safety` expose platform identity, Windows access paths and known system/boot/recovery/hidden/read-only/offline flags. Unknown flags remain null.
- Physical disks expose `disk_uuid` and `mbr_signature` where reported.
- Partitions expose `number`, `gpt_type`, `mbr_type`, `active`, `shadow_copy`, `no_default_drive_letter` and `volumes`.
- Each Windows volume has a separate `device` and optional `label`. A partition's type, UUID, offset and size are separate from its volume's filesystem, label, drive letter and available capacity.
- `unattached_volumes` preserves volumes whose physical backing is not represented, including unsupported storage configurations. These are inspection-only.

macOS retains normalized APFS containers, physical-store links, volumes, quota/shared capacity and mounted snapshots. APFS containers and their physical stores have no mount/unmount actions. Windows uses structured Storage-module `Get-Disk`, `Get-Partition` and `Get-Volume` output. Per-disk discovery failures become warnings. Failed top-level queries return errors. Legacy `diskvio_list_disks_json()` still uses its independent disk-only query, so partial volume-discovery failures do not break the existing Windows disk list.

## Operations and JSON ABI

All non-null returned strings belong to Rust. Copy UTF-8 and free each allocation exactly once with `diskvio_string_free`. A request is borrowed for the duration of the call and must contain 1 to 16384 readable bytes. The signatures of all four C exports are unchanged, including pointer-sized `size_t` request lengths. Rust still produces a static library and a dynamic library.

The existing execution request continues to work:

```json
{"action":"mount","identifier":"disk2s1","expected_identity":"<inventory identity_token>"}
```

The optional `mode` field defaults to `execute`. Two read-only modes use the same `diskvio_operation_json` export:

```json
{"mode":"supported_operations","identifier":"disk2s1"}
```

```json
{"mode":"validate","action":"mount","identifier":"disk2s1","expected_identity":"<inventory identity_token>"}
```

Success responses contain `operation`, `capabilities` or `validation`, respectively. Capabilities include `identifier`, `device_kind`, `identity_token` and `actions`. Successful validation includes `valid:true`, action, the resolved current identifier and expected identity. Validation never reserves a device or authorizes a later execution; execution performs its own checks. Failed validation returns `status:error` and `validation.valid:false`.

Errors retain the frontend-compatible top-level `message` and add structured details:

```json
{"status":"error","message":"Access denied","error":{"code":"permission_denied","message":"Access denied","platform_code":5}}
```

Codes include `invalid_request`, `missing_target`, `ambiguous_target`, `identity_changed`, `protected_device`, `unsupported_operation`, `invalid_state`, `incomplete_discovery`, `permission_denied`, `busy`, `command_failed`, `io`, `unsupported_platform` and `operation_in_progress`. Native exit/error codes are preserved where available. Process stderr/stdout diagnostics remain visible; diagnostic classification is best effort.

Rust callers can use `supported_operations`, `validate_operation`, `perform_operation`, or the inventory-based query/validation helpers. Requests and results are strongly typed. `DiskBackend` owns discovery and execution, with shared validation before dispatch. Platform command execution and Windows volume management can be replaced with fixtures in tests.

## Safety policy

Operations use a freshly discovered inventory and recomputed capabilities. Caller-provided actions and identity tokens stored in a supplied snapshot are never trusted as policy. Execution performs another inventory/identity check and platform checks immediately before the OS call. Stable identity tokens can resolve a volume after a BSD-name change. Replacement devices, missing targets, duplicate identifiers or UUIDs, incomplete discovery and changed mount state are rejected.

Internal or unknown-location disks are inspection-only. System, boot and recovery devices block management of their backing disks. macOS system APFS roles, root/system paths and mounted system snapshots are protected, including external installations. Every store in a multi-store APFS container must resolve to a verified external disk. Locked/snapshot-mounted APFS volumes and unmodeled CoreStorage/RAID pools have no volume operations. Ejection is supported only for verified external macOS physical disks reported as ejectable or removable.

macOS uses absolute `diskutil`/`ioreg` paths, separate arguments, closed stdin, UUID targets where available and IOMedia registry identities. Only mount, unmount and eject commands can execute. No force option, shell, sudo, privileged helper or automatic retry exists.

Windows management is limited to verified USB disks with known safe flags and stable storage identifiers. Supported volumes must have a valid volume GUID path, a basic GPT or recognized basic MBR partition, and NTFS, FAT or exFAT. Hidden, recovery, active/boot, offline, read-only, shadow-copy and unsupported volumes remain inspection-only. Directory mounts and multiple access paths are inspection-only. Native mount/unmount uses volume GUID paths rather than drive letters as identities:

- Mount assigns the first available drive letter from D through Z with `SetVolumeMountPointW` after verifying there are no existing access paths.
- Unmount verifies the single drive letter and its volume GUID, acquires `FSCTL_LOCK_VOLUME`, checks access paths again, dismounts the locked volume, and removes the drive letter. If locking fails, no dismount or mount-point removal occurs. The lock remains held until the handle closes, including on failure.
- Windows eject is explicitly unsupported. This milestone does not take disks offline or substitute dismounting for safe hardware removal.

Windows discovery retains its 30-second timeout, concurrent pipe draining and hidden-console behavior. Native access errors are returned without elevation. Windows may require additional access rights for these APIs; Diskvio does not request those rights itself.

No software snapshot makes metadata checking and OS dispatch atomic. Devices may disappear between checks; OS failures are returned and operations are never retried automatically. Removing a Windows drive letter does not disable later OS remounting through other means. If removal fails after a successful dismount, the operation reports failure and the remaining drive letter can allow Windows to remount the filesystem when accessed.

Primary Windows references: [Storage partition metadata](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-partition), [volume locking](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_lock_volume), [dismount behavior](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_dismount_volume), and [mount-point assignment](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setvolumemountpointw).

## Frontend integration and remaining native work

The existing macOS action menus already consume Rust's `actions` and identity tokens. Swift `DiskService` additionally exposes supported-operation queries and validation, and preserves structured backend error details. Existing requests/outcomes remain compatible. No app restructuring or new UI actions are required on macOS.

The existing Windows `NativeMethods`, `DiskService` and `IDiskService` now expose richer inventory, capability queries, validation and execution. C# models preserve capacities as nullable `ulong`, identify volumes separately and decode structured errors. JSON allocations use the existing SafeHandle ownership path. DLL loading verifies inventory/operation/free exports before allocating a response.

The WinUI page still displays the legacy physical-disk list. Partition/volume presentation and action controls are deliberately left for a Windows-native implementation and verification pass. In Visual Studio 2026, bind `InventoryAsync()` to the existing device view, select a volume, show only its backend-reported actions, submit its current `IdentityToken` through `PerformAsync`, serialize operations with refresh, and refresh after both success and failure. Do not infer capabilities from filesystem or drive letter in C#. Windows eject must remain unavailable. Verify the XAML layout, x64 P/Invoke signatures, MSIX DLL packaging and real Storage-module output before enabling those controls.

## Verification

On 9 October 2026, the Apple Silicon Mac passed:

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`, also with `--all-targets`
- `cargo test --workspace`: 41 tests, with fixtures and mocked operation commands only
- Windows x64 cross-check and Clippy, including test targets
- Xcode Debug compilation/static-library linking and 14 Swift unit tests
- 23 portable C# contract/service/view-model tests, with the native DLL test excluded

The Windows C# contract, service and view-model tests can run without WinUI or a native DLL:

```sh
dotnet test windows/Diskvio.Windows.Tests/Diskvio.Windows.Tests.csproj \
  -p:DiskvioPortableTests=true --filter 'Category!=NativeIntegration'
```

The portable mode changes only the test project target and omits the Rust DLL build/package target. Normal Windows builds retain the existing x64 DLL pipeline. The `NativeIntegration` test must be excluded in portable mode.

The Mac cannot link a Windows MSVC DLL or run Windows kernel32/Storage-module calls, WinUI or MSIX deployment. Cross-compilation and fixture tests do not constitute Windows runtime or removable-hardware validation. No actual disk-management operation was executed during verification. External-device mount/unmount/eject success, permission failures and busy-file behavior still need controlled hardware verification on each native OS.
