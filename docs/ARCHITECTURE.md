# Architecture

The existing Tauri 2 / React desktop shell, six themes, native menus and component boundaries are retained. One root Cargo workspace and pnpm workspace own their lockfiles. `diskvio-helper` remains an inactive, non-privileged executable that is neither bundled nor invoked.

## Discovery flow

1. Native builds default to Live Mode. Settings can explicitly choose Demonstration Mode; only native builds persist that preference. Browser preview always uses labelled local fixtures.
2. `get_app_info` reports the host OS and compiled discovery support; permission/provider availability is determined by `list_disks`, rather than claiming successful enumeration from a capability boolean.
3. The no-argument `list_disks` command moves blocking discovery to Tauri's blocking pool. Platform-specific implementations live behind `DiskPlatform`. Failures return `{code, message}` (`unsupported`, `permissionDenied`, `discoveryFailed`, `invalidData`, `timeout`), never a successful empty inventory. A genuinely empty enumeration is valid.
4. Rust validates unique disk/partition/volume identities, positive capacities, sorted non-overlapping geometry, bounds, checked arithmetic and reliable usage bounds before IPC. Missing serial numbers are valid for read-only display.
5. React holds one source and inventory. Request generations reject obsolete refresh/source-switch results. A refresh keeps previous data visible, explicitly marks failures as stale, and reconciles the selected disk against identity, serial, model and capacity. Removal/replacement clears selection and explains why. Source switching clears all inventory/selection before loading. There is no polling.

## Domain and byte transport

`Disk` is an OS-visible physical device, `Partition` owns exact physical offset/length, and `Volume` owns filesystem metadata, labels, mounts and optional usage. APFS logical volumes belong to backing partitions and are marked `sharedSpace`; they are never promoted to physical devices. A volume's filesystem capacity can be smaller than its partition. Free filesystem bytes are not calculated as partition length minus used bytes.

Every Rust `u64` byte count serializes as a decimal **string**, including optional volume counts (`null` when unknown). TypeScript uses `bigint` for arithmetic and only converts final display proportions to floating point. Fixture JSON is deserialized and validated by Rust tests. Even `u64::MAX` roundtrips exactly.

`partitions: null` means geometry is unavailable; `[]` means a known empty layout. Only a complete validated layout permits gap/unallocated calculations. Partition maps use actual offsets with no artificial minimum widths; table rows keep tiny EFI/recovery regions identifiable. Totals are unknown if any filesystem usage is missing or shared. The table shows separate used/free figures; shared APFS usage is never summed across physical stores.

## Platform adapters

### Windows

The maintained [`wmi` Rust crate](https://docs.rs/wmi/0.18.4/wmi/) queries native COM/WMI in `ROOT\Microsoft\Windows\Storage`: [MSFT_Disk](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-disk), [MSFT_Partition](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-partition), [MSFT_Volume](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-volume). Queries select fixed property sets; no methods, raw handles or PowerShell are used. Partition `AccessPaths` match volume GUID paths. Windows marks NVMe/ATA/SATA as internal, USB/1394 as external; ambiguous buses remain unknown. Explicit virtual/file-backed/Storage Spaces buses are excluded. RAID devices are shown as Windows presents them; member-drive topology is not invented.

If the modern namespace is inaccessible, fixed `ROOT\CIMV2` queries use `Win32_DiskDrive`, `Win32_DiskPartition`, `Win32_LogicalDisk` and their partition/volume association. The fallback reports limitations; online/read-only state and modern bus details can be unknown. Extended MBR container layouts are unavailable instead of overlapping physical extents. Empty card readers with no positive media capacity are excluded. Modern GUID paths can expose only volume metadata Windows permits the caller to read. Recovery/unmounted/locked/non-Windows filesystems can lack labels, filesystem type or usage.

Selection identity prefers `MSFT_Disk.UniqueId`, otherwise a device path plus available serial/model/capacity; compatibility discovery prefers PNP identity. OS disk numbers alone are not treated as durable identities. Bus/classification may legitimately change when switching provider. Native APIs may expose virtual devices on otherwise ordinary buses; classification is conservative, not a write-safety guarantee.

### macOS

Fixed absolute `/usr/sbin/diskutil` calls use only `list -plist physical`, `info -plist <validated BSD id>` and `apfs list -plist`. BSD identifiers must match `diskN(sN)*`; arguments are separate process arguments, never shell text. Plists are parsed with the Rust `plist` crate. Physical listings exclude synthesized APFS disks. APFS `PhysicalStores` associate logical volumes with backing partitions. Missing offsets or inaccessible partition information makes the layout unavailable, not a guessed sequence of extents.

Identity prefers disk UUID, then available serial/model, then BSD path/model/capacity. macOS often does not expose hardware serials through diskutil, so path fallback cannot prove continuity across indistinguishable replacement devices. APFS shared stores, snapshots, quotas, Fusion/multi-store configurations and native runtime require host testing. System classification remains unknown where it cannot be proved, excluding those devices from USB candidates.

### Linux

Fixed absolute `/usr/bin/lsblk` (or `/bin/lsblk`) runs [`--json --bytes --tree` with an explicit column list](https://www.man7.org/linux/man-pages/man8/lsblk.8.html). Kernel names and canonical sysfs paths filter out `/sys/devices/virtual` nodes, loops, device mapper and logical volumes as physical disks. Only direct `part` children supply physical extents; nested filesystems are marked shared. `START` is in 512-byte kernel sectors and is checked before multiplication; `SIZE` is bytes. Mount paths come from `MOUNTPOINTS`, not names inferred from labels. WWN, then serial/model, then device path identify a disk. Root/boot mounts protect USB classification; unknown transport stays unknown.

The util-linux utility is used because it combines sysfs/udev topology, filesystem metadata and mounted filesystem statistics in one documented structured read-only interface. Root is not required; permissions and unsupported util-linux columns can yield unavailable properties or a structured failure. Whole-device filesystems currently report the partition layout as unavailable. Filesystem/container topology and hardware runtime need Linux host verification.

Both Unix utility calls have a 20-second deadline and 16 MiB output limit. Exit/parse failures are explicit; utility stderr is not copied into logs. No shell interpreter is used.

## Shell and safety boundaries

Native menus retain navigation, Settings, About and Quit conventions; Cmd/Ctrl+R refreshes the active inventory. Event subscriptions are allowlisted and cleaned up. The 1240 × 820 default / 800 × 600 minimum window remains; larger real inventories use accessible scrolling tables/lists. Themes remain System, Light, Dark, Ocean, Forest and Sand.

The main-window capability still permits only native event listen/unlisten. Custom application commands are `get_app_info` and `list_disks`. No shell, filesystem, opener or dialog plugin, automatic upload, elevation or privileged helper is introduced. Discovery capabilities describe read support only; partition management and USB writing are always false. USB choices require a known external/removable USB connection, non-system status and no known offline/read-only condition. No candidate is automatically selected, and writing remains disabled.

Identity validation supports display continuity only. Devices without serials, cloned identifiers and path reuse cannot meet future destructive-operation requirements. Before any future write, independent backend safety validation, capability checks, stronger fresh identity evidence and explicit authorization remain mandatory. See [Safety](SAFETY.md).
