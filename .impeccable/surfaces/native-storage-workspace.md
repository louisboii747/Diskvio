# Native Storage Workspace

Mode: Operate

Diskvio helps a person inspect connected physical disks and their real partition and filesystem relationships, then explicitly invoke supported management actions on verified external targets. The working loop is browse or search, select, compare the map and rows, inspect exact metadata, and refresh or choose an available command.

The surface uses a grouped device browser, selected-device overview, filesystem-aware physical map, sortable rows and an information inspector. Search directly exposes matching child devices with their parent disk. The interface stays native to SwiftUI on macOS and WinUI 3 on Windows, using the shared design rules in [DESIGN.md](../../DESIGN.md).

Keep unavailable facts visible and operation eligibility tied to Rust capabilities. APFS shared capacity, unmapped metadata and Rust-reported allocation regions have distinct meanings. Windows retains a separate preview-only planning expander; macOS presents unsupported planning honestly. No destructive execution is part of this surface change.

Discovery, refresh, incomplete metadata, errors, empty results and device removal need a visible state. Selection should retain stable identity across refresh and reconcile when a target disappears. Pane resizing, narrower layouts, keyboard navigation and text copying serve sustained inspection.

The current evidence and outstanding native checks are recorded in [docs/native-ui-redesign.md](../../docs/native-ui-redesign.md). Source implementation alone does not certify every appearance, accessibility or physical-device interaction.
