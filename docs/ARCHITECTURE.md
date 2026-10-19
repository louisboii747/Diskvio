# Architecture

Diskvio is a single Tauri 2 desktop application with a React/TypeScript interface and an unprivileged Rust backend. Milestone 1 is implemented. All displayed devices are local fixtures.

## Responsibilities

| Path                          | Responsibility                                                                                                |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `apps/desktop/src/components` | Desktop shell, shared states and official shadcn UI components                                                |
| `apps/desktop/src/features`   | Disk overview, partition inspector, USB workflow and settings                                                 |
| `apps/desktop/src/models`     | Typed frontend contracts and navigation                                                                       |
| `apps/desktop/src/fixtures`   | Explicitly simulated device inventory                                                                         |
| `apps/desktop/src/services`   | Local fixture loading and the native IPC/event boundary                                                       |
| `apps/desktop/src/hooks`      | Async resources and persisted/system appearance                                                               |
| `apps/desktop/src-tauri`      | Window, native menus and the sole `get_app_info` command                                                      |
| `crates/diskvio-core`         | Serializable disk/partition models, layout validation, preliminary identity checks and SI capacity formatting |
| `crates/diskvio-platform`     | `DiskPlatform` trait, capability descriptions and unavailable adapter                                         |
| `crates/diskvio-helper`       | Inactive, non-privileged executable; not bundled or invoked by the app                                        |
| `assets/branding`             | Original SVG icon source                                                                                      |
| `.github/workflows`           | Windows, macOS and Linux compile/test matrix                                                                  |

There is one root Cargo workspace and lockfile; the Tauri crate has no nested workspace. The pnpm workspace contains the desktop application and one root lockfile. Release settings live at the Cargo workspace root.

## Implemented execution flow

1. The frontend invokes `get_app_info` through Tauri IPC. Rust returns version, platform, demo mode and capabilities.
2. The native Rust menu emits an allowlisted action to the main window. React subscribes, navigates or reloads local fixtures, and releases its listener on unmount (including Strict Mode remounts).
3. Fixture loading is isolated behind an asynchronous service with loading, empty, error and retry states. Refresh does not scan hardware. Selection is shared through the app shell using ordinary React state; Zustand is unnecessary at this size.
4. The same selected disk is used in the overview and partition inspector. USB target choices include only simulated removable devices.
5. Partition widths derive from byte capacities and offsets, including gaps. No minimum-width distortion is applied; tables and legends make tiny partitions readable. Capacity units are decimal SI, and free space within partitions is distinct from unallocated space.
6. The platform adapter denies all capabilities and returns `NotYetAvailable` for discovery. It does not report an empty list as if discovery succeeded.

Rust tests deserialize the actual frontend fixture JSON and validate its layouts, checking the serialization contract without a second Rust fixture source. Frontend models currently mirror the Rust camelCase types; generated contracts can be considered when live discovery expands. Keep byte values within JavaScript's safe integer range when adding frontend fixtures; future larger-than-safe-integer capacities need a string/bigint transport contract.

## Native shell and permissions

The window is 1240 × 820 logical pixels by default, minimum 800 × 600. System decorations remain native. The UI adapts to smaller browser previews as well.

The Rust menu uses Tauri 2 menu builders. macOS gets Diskvio, File, Edit, View, Disk, Partition, USB, Window and Help menus, including application services/hide/quit conventions and explicit Window/Help roles. Windows/Linux use File for Quit and View for Settings. Standard Edit and Window actions use native predefined items. Destructive menu items are disabled.

Navigation shortcuts: Cmd/Ctrl+1 (overview), +2 (partitions), +3 (USB); Cmd/Ctrl+R reloads demonstration data; Cmd/Ctrl+, opens Settings. About uses Tauri's native predefined About dialog. Frontend shortcuts are supplied by native menus, not a duplicate HTML menu bar. macOS/Linux runtime behavior and native menu click-through require platform testing.

The main-window capability permits only native event listen/unlisten. The sole custom app command returns metadata. No shell, opener, filesystem or dialog plugin is installed, no helper is launched, and no storage operation is exposed. The CSP restricts scripts/assets to the application and IPC; development additionally allows the local Vite connection. The local font is packaged, not fetched remotely.

## Future execution flow and platform boundaries

Live discovery belongs in platform-specific, read-only adapters behind `DiskPlatform`: native Windows storage APIs, macOS disk facilities and Linux storage facilities. Opening the UI must remain unprivileged.

Before future writes, the core must validate an operation against a fresh identity snapshot and explicit capabilities. Only a deliberate, authenticated, narrowly typed request may reach a separately reviewed helper. The helper must re-discover and revalidate the target after authorization, report progress/errors and deny unsupported work. The current preliminary identity comparison is **not** a complete write authorization mechanism.

Never construct shell commands from user paths, run the entire UI as administrator/root or expose generic privileged execution. See [Safety](SAFETY.md).
