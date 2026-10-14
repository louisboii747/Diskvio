# Native UI redesign

The redesign keeps Diskvio's native identity and shared Rust backend while improving device discovery, inspection and selection on SwiftUI and WinUI 3. Product data still comes from real backend inventory. Rust remains authoritative for operation safety, geometry, planning and the stable JSON/C ABI. No disk formatting, erasing, partition mutation or raw writing was added or performed.

## macOS changes

- The sidebar groups devices, filters reported internal/external/removable targets, and searches names, identifiers, filesystem and connection metadata. Active search directly shows matching partitions and volumes with their parent disk.
- The overview shows reported model, connection, scheme, status, protection badges and capacity. Exact-byte tooltips and a usage indicator preserve precision without crowding primary values.
- The inspector groups general, storage, connection, protection and advanced properties, with selectable text and copy actions. Missing metadata and APFS shared capacity stay explicit.
- The filesystem map, sortable partition rows, sidebar and inspector share selection. Tiny partitions keep minimum widths. Left/Right navigation moves focus and selection from the focused segment and scrolls the destination into view. Context menus retain existing supported operations.
- Native split panes, an adaptive inspector popover, semantic light/dark colors, reduced-motion-aware selection feedback and loading/error/disconnection notices complete the workspace behavior.

## Windows changes

- The WinUI title bar and command bar lead a grouped browser, searchable child results, device filters, overview and grouped information pane. Unattached volumes remain separately inspectable.
- The sidebar resizes by pointer or keyboard and moves above the working area below the compact threshold. The information pane moves below content when it cannot fit alongside. A wide partition table scrolls horizontally.
- Filesystem theme brushes support Light, Default and HighContrast dictionaries. Native controls retain their own hover, focus and motion. Selected map targets add a checkmark and accent border.
- Search, map, rows, sorting and context menus preserve the associated physical disk and selected identity. Exact-byte tooltips, honest unknown capacities, progress, retryable errors and disconnection notices expose real state.
- Existing preview-only partition planning remains in an expander. Proposed layouts, backend allocation regions and unmapped metadata remain distinct; no Apply or Commit operation was introduced.

## Validation scope

Validation below was performed during this redesign session on macOS. Portable Windows checks do not validate native WinUI rendering or execution.

| Scope | Result | Evidence and limits |
| --- | --- | --- |
| Rust formatting, check, Clippy and tests | Passed | 66 core, 31 partition-planning and 9 FFI tests. Backend code and ABI remain unchanged by the redesign. |
| Portable Swift presentation suite | Passed after final interaction corrections | 23 tests covering presentation, matching and map widths. |
| SwiftLint and whitespace checks | Passed | Strict SwiftLint and `git diff --check`. |
| Native macOS model/state tests | Passed | 25 tests. This is distinct from the UI test target. |
| macOS Debug build | Passed after final interaction corrections | Native arm64 build and 25 native tests. |
| macOS Release build | Passed after final interaction corrections | Native arm64 build with `SWIFT_TREAT_WARNINGS_AS_ERRORS=YES`. |
| Portable Windows tests | Passed after final interaction corrections | 98/98 tests with `TreatWarningsAsErrors`, `EnforceCodeStyleInBuild` and latest-recommended analyzers. |
| WinUI XAML and binding/event inspection | Passed static checks after final corrections | All current XAML re-parsed and workspace named handlers resolved; bindings were inspected against source. This does not compile XAML with the Windows toolchain. |
| Native WinUI build on this Mac | Failed due to unavailable host toolchain | MSVC `link.exe` is unavailable on macOS. Native Windows compilation is unverified. |
| macOS UI smoke target | Not executed | Added light/dark workspace and search/clear smoke coverage and included it in the shared scheme. The runner was killed before bootstrap in unsigned and ad hoc signed attempts. No UI test pass is claimed. |
| Xcode RenderPreview | Failed to launch | App launch timed out. Preview rendering is unverified. |
| Actual macOS interaction via CUA | Observed in light appearance | Overview, map/list/inspector selection, empty search and clear, direct nested-volume search selection with matching inspector and parent-store highlight, and fullscreen were inspected. Screenshots are in `.impeccable/review/`. |
| Remaining runtime scope | Unverified | Compact resizing, dark runtime, VoiceOver, enlarged text, Windows native rendering/DPI/accessibility and physical external-device operations. |

The focused reviewer pass returned `ship` for the hidden-search and map-focus corrections. Search was verified in source and a fresh screenshot; keyboard focus divergence was corrected in source and compiled, but CUA Tab did not establish map focus under the current system settings. Runtime keyboard navigation remains unverified. This focused verdict and the observed light macOS state are not whole-product visual approval. The failure to bootstrap UI tests also leaves their new smoke assertions unexecuted.

## Changed-file inventory

The inventory includes tracked changes, new source/tests, this documentation and grouped screenshot artifacts.

| Area | Files |
| --- | --- |
| macOS window and scheme | `macos/Diskvio/Diskvio/ContentView.swift`; `macos/Diskvio/Diskvio.xcodeproj/xcshareddata/xcschemes/Diskvio.xcscheme` |
| macOS models | `macos/Diskvio/Diskvio/Models/DeviceBrowser.swift` (new); `macos/Diskvio/Diskvio/Models/Disk.swift`; `macos/Diskvio/Diskvio/Models/PartitionMapLayout.swift`; `macos/Diskvio/Diskvio/Models/PartitionPresentation.swift`; `macos/Diskvio/Diskvio/Models/WorkspaceLayout.swift` |
| macOS state | `macos/Diskvio/Diskvio/Services/DiskStore.swift` |
| macOS views | `macos/Diskvio/Diskvio/Views/DeviceCommands.swift`; `macos/Diskvio/Diskvio/Views/DeviceDetailView.swift`; `macos/Diskvio/Diskvio/Views/DeviceInspector.swift`; `macos/Diskvio/Diskvio/Views/DeviceSidebar.swift`; `macos/Diskvio/Diskvio/Views/DeviceWorkspaceView.swift`; `macos/Diskvio/Diskvio/Views/PartitionMapView.swift` |
| macOS tests | `macos/Diskvio/DiskvioUITests/DiskvioUITests.swift`; `macos/Tests/PresentationTests/PresentationTests.swift` |
| Windows shell and palette | `windows/Diskvio.Windows/App.xaml`; `windows/Diskvio.Windows/MainWindow.xaml`; `windows/Diskvio.Windows/MainWindow.xaml.cs` |
| Windows map and models | `windows/Diskvio.Windows/Controls/PartitionMap.cs`; `windows/Diskvio.Windows/Models/PartitionLayout.cs`; `windows/Diskvio.Windows/Models/PartitionPresentation.cs` |
| Windows state and presentation | `windows/Diskvio.Windows/ViewModels/StorageNode.cs`; `windows/Diskvio.Windows/ViewModels/StorageWorkspaceViewModel.cs`; `windows/Diskvio.Windows/ViewModels/StorageWorkspaceViewModel.Presentation.cs` (new) |
| Windows workspace | `windows/Diskvio.Windows/Views/DevicesPage.xaml`; `windows/Diskvio.Windows/Views/DevicesPage.xaml.cs` |
| Windows tests | `windows/Diskvio.Windows.Tests/BrowserTests.cs` (new) |
| Design and report | `DESIGN.md`; `.impeccable/design.json`; `.impeccable/surfaces/native-storage-workspace.md` (new); `docs/native-ui-redesign.md` (new) |
| Visual evidence | `.impeccable/review/` screenshot artifacts, including `macos-light-overview.png`, `macos-light-selection.png` and `macos-light-search.png` |

## Next native checks

1. Run the macOS UI target on a host where its runner starts. Exercise dark appearance, compact window sizes, sidebar and inspector limits, search-result selection, map keyboard navigation, reduced motion, VoiceOver and enlarged text.
2. Build and run the WinUI app on Windows with MSVC. Inspect Light, Dark and HighContrast themes, different DPI/text scales, compact resizing, keyboard focus, sorting, search, context menus and inspector placement. Run `python scripts/check.py windows --native-integration` there.
3. Validate loading, partial-metadata, refresh failure and disconnection behavior on native hosts. Separately validate supported external-device operations on suitable media through existing confirmations. Keep all destructive disk changes outside this redesign's scope.
