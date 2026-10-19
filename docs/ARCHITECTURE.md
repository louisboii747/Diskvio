# Architecture

Diskvio is planned as a **single Tauri 2 desktop application** with a React/TypeScript interface and Rust-based disk logic. This is a design target, not an implementation claim.

## Responsibilities

| Path | Responsibility |
| --- | --- |
| `apps/desktop/` | Vite + React application, Tauri shell, UI state, menus and OS-facing commands |
| `crates/diskvio-core/` | Disk/partition models, unit conversions, capability descriptions and operation validation |
| `crates/diskvio-platform/` | OS-specific adapters for read-only discovery and supported disk functions |
| `crates/diskvio-helper/` | Narrow, authenticated elevated operations; not a general-purpose command runner |
| `assets/branding/` | Shared icon and brand source assets |
| `docs/` | Architecture, safety and future decisions |

## Intended execution flow

1. The React frontend invokes explicit Tauri commands and receives serialisable, typed results.
2. The unprivileged Rust side discovers devices and describes supported operations without changing data.
3. The core validates proposed changes using a device identity snapshot and operation capabilities.
4. Only a deliberate, authorised action can reach a privileged helper. The helper re-discovers and revalidates the target before execution.
5. Operation progress, errors and completion status are returned to the UI.

## Platform boundaries

- **Windows:** platform-native storage APIs, supported elevation mechanisms and native Windows window behaviour.
- **macOS:** platform disk facilities, standard application menus (including planned Disk/Partition/USB entries) and OS-authorised privileged operations.
- **Linux:** platform disk facilities, mount/volume support and appropriate polkit or equivalent authorisation where needed.

Avoid constructing shell commands from user-provided paths, launching an entire UI as administrator/root, or exposing generic privileged command execution through Tauri.

## Scaffold milestone

The first implementation milestone will add actual buildable code to `apps/desktop/`, establish Cargo crates/workspace membership, and update this document with the **real** structure and verified build commands. Retained `.gitkeep` files can be removed as the directories gain files.
