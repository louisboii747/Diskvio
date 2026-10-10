# Milestone 3 — Native macOS disk management integration

Milestone 3 connects Diskvio's existing Rust mount, unmount and eject operations to the native macOS SwiftUI application. The interface remains inspection-first: the Rust backend is the sole authority for capability, identity and safety validation, and no formatting, partition editing or image-writing UI is present.

## Implemented features

- Mount and unmount actions are shown only on devices whose inventory entry contains the corresponding backend-reported action.
- Eject is available only for a backend-approved external physical disk. Selecting one of its partitions, containers or volumes can still expose the parent disk's eject action.
- Sidebar and child device rows use native SwiftUI context menus for applicable operations, Finder access, identifier copying and refresh.
- Native **Device**, **Partition**, and **USB** application menus mirror the window's selected-device commands. Device provides Finder access, identifier copying and refresh; Partition provides mount/unmount; USB provides mount/unmount/eject for the selected external USB hierarchy. Unavailable actions are disabled. Refresh uses Command-R and eject uses Shift-Command-E.
- Unmount and eject require explicit confirmation. Their dialogs explain that open files and mounted volumes can become unavailable. Mount remains an explicit user action but does not add a second confirmation.
- Operations show progress and a success or failure notice. Backend error messages are presented as user-facing text rather than raw JSON or error objects.
- Successful and failed operations both trigger a fresh inventory scan. Stable node identities preserve selection when possible; disappearance handling removes stale devices immediately and reconciles selection.
- Only one refresh or disk operation can run at a time. Discovery, capability queries, validation and execution run away from the main actor.
- Existing disk details remain available: device name and identifier, media/model name where macOS reports it, capacity, connection, internal/external, removable/ejectable state, partition scheme and type, filesystem, mount point, APFS container/volume data, usage, encryption, lock state, roles, quota, reserve and mounted snapshots.

## Architecture changes

`DiskStore` remains the window-scoped source of truth. It now coordinates a complete operation pipeline:

1. The SwiftUI control checks the actions embedded in the current Rust inventory snapshot.
2. Unmount/eject is staged as a `PendingDiskOperation` and requires confirmation.
3. Immediately before execution, the store calls the FFI capability query for the selected identifier.
4. The store calls the FFI validation endpoint using the identity token from the inventory the user actually selected.
5. The execution request uses that same identity token. Rust performs its own fresh discovery and validation again before dispatching to macOS.
6. The store refreshes the inventory and reconciles selection after either success or failure.

This deliberately does not replace or duplicate Rust safety policy in Swift. The UI consumes backend actions, while Rust continues to reject internal, protected, stale, ambiguous, unsupported or invalid-state targets.

The active window publishes its `DiskStore` through SwiftUI focused-scene state. The command types read that store to build real macOS `CommandMenu` instances, preserving the standard Diskvio, File, Edit, View, Window and Help menus.

## FFI integration

The integration uses the existing exports only:

- `diskvio_inventory_json()` for topology, capabilities and identity tokens.
- `diskvio_operation_json()` with `supported_operations`, `validate` and the default execution mode.
- `diskvio_string_free()` for Rust-owned response strings.

No ABI changes were needed. Operation responses continue to decode into typed Swift values, including structured backend errors. No raw JSON reaches the interface.

## Automated verification

Completed on 10 October 2026:

- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo clippy --workspace -- -D warnings` — passed.
- `cargo test --workspace` — passed, 41 tests.
- Xcode build-for-testing — passed.
- Swift unit and UI tests — passed, 20 tests.

The added Swift tests cover backend-reported action availability, external USB selection, confirmation gating, successful execution and refresh, changed capabilities, unsupported execution prevention, validation error propagation and existing selection/disappearance behavior. Every operation dependency is injected with fixture closures; automated tests never call the real mount, unmount or eject backend.

## Manual external USB testing

Use only an explicitly selected expendable external USB disk. Never use the Mac's internal/system disk or media containing needed data.

1. Save other work, connect the expendable USB disk, and launch Diskvio normally.
2. Select the physical disk and verify its identifier, capacity, connection (`USB`), external/removable status and partition map before using any action.
3. Select an unmounted supported volume. Confirm that Mount appears in its context/toolbar/USB menu and that Unmount is unavailable. Choose Mount and verify the mount point appears after refresh.
4. Select the mounted volume. Choose Unmount, verify the warning dialog, cancel once, then repeat and confirm. Verify the mount point and available actions update.
5. Mount the volume again, select the external disk or one of its descendants, choose Eject, verify the whole-disk warning, and confirm only when no files are open. Verify the device disappears from the sidebar.
6. For error handling, repeat unmount while a disposable file on the test volume is deliberately held open. Confirm that Diskvio reports the backend/macOS error and remains responsive. Do not force the operation.
7. Reconnect the disk and verify selection, context menus and USB menu actions follow the currently selected device.

No manual operation should be attempted unless the displayed identifier and device details match the expendable test disk.

## Known limitations

- Real-device success, permission, busy-volume and disconnect races require the controlled manual procedure above; they are intentionally not automated.
- macOS determines whether an operation needs additional privileges. Diskvio does not use `sudo`, a privileged helper, force flags or automatic retries.
- Manufacturer is shown only when macOS supplies it through the existing media/model fields; the backend does not currently expose a separate manufacturer field.
- APFS containers, physical stores, locked volumes, mounted snapshots, internal/system disks and other backend-protected configurations remain inspection-only.
- Formatting, erase, partition create/delete/resize and ISO writing remain out of scope.
- Windows UI functionality is unchanged.
