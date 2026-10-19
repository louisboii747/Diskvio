# Disk operation safety principles

Diskvio deals with operations that can irreversibly remove user data. Milestone 1 uses explicitly labelled local demonstration fixtures only. The following remain **requirements for future disk operations**, not assurances that those operations are implemented.

## Default behaviour

- Prefer read-only device discovery without elevated permissions.
- Distinguish physical devices, partitions, volumes, filesystems and mount points.
- Never automatically execute a destructive action just because an item is selected.
- Never report a simulated or queued operation as a successful real disk write.

## Before any write

1. Show the exact target disk, capacity, device identifier and operation effects.
2. Explicitly communicate when data will be erased or changed.
3. Identify mounted volumes, system/boot disks, protected media and unsupported configurations.
4. Require an intentional, specific confirmation for destructive work.
5. Re-enumerate and revalidate the target identity immediately before execution, including after privilege elevation.
6. Stop safely if the device changed, was removed, is locked or cannot be reliably identified.

## Privilege model

Keep ordinary UI and read-only operations unprivileged. Use OS-supported authorisation and a **minimal privileged helper** for unavoidable elevated work. Accept narrowly typed requests, not arbitrary commands or paths; reject unsupported operations and untrusted targets. Never rely solely on a frontend confirmation as a security boundary.

## Testing and release gates

- Unit-test validation and device identity checks.
- Run destructive integration tests only against explicit disposable devices or isolated fixtures.
- Never run destructive tests against a developer's system or internal disk.
- Verify cancellation, error reporting and interrupted-operation behaviour; do not promise that every filesystem write can be rolled back.
- Document platform-specific limitations before enabling any action in production.

No destructive disk operation has been implemented in this repository yet.

## Milestone 1 enforcement

- The only registered application IPC command is `get_app_info`, which returns metadata and unavailable capabilities.
- The platform adapter performs no discovery and denies every disk capability.
- Destructive buttons and native menu items are disabled. Refresh reloads local fixtures only.
- The helper is a non-privileged placeholder, rejects all arguments, and is not bundled or launched by the desktop app.
- No shell, filesystem, opener or file-dialog plugin is installed. Main-window permissions allow only event subscription and cleanup.
- No elevation, administrator startup requirement, raw disk access, mount/unmount or eject code exists.
- Tests exercise fixtures and pure models; they do not inspect or modify developer storage.
