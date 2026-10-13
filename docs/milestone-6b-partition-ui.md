# Milestone 6B: Native partition workspace and interactive planning

Date: 13 October 2026.

Windows provides configuration and preview workflows over the Milestone 6A Rust planner. macOS improves native inspection and presents its unsupported planning capability explicitly. Partition creation, deletion, formatting, resizing and raw disk writes remain unavailable. No partition execution API, dispatcher or Apply control was added.

## Windows workspace

The existing Devices page, `StorageWorkspaceViewModel`, `StorageNode` hierarchy and asynchronous `DiskService` remain the architecture. Planning state lives in a partial definition of the same view model. `PartitionPlanningView` supplies native controls in the scrollable detail pane; only test-media designation uses a confirmation dialog. Existing Mica, typography, theme brushes, icons and volume commands remain.

The disk overview reports name/number, exact capacity, manufacturer/model, connection, scheme, partition count, online/read-only status, unallocated capacity and eligibility. Manufacturer/model are optional additive inventory fields populated from existing read-only [MSFT_Disk properties](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-disk). Missing values remain explicitly unreported. No extra hardware query is required.

Before planning discovery, map gaps retain their metadata/unmapped meaning. Inspect backend layout requests authoritative Rust discovery. Disk size, partition count, identities, extents and types must agree with inventory before regions enter the explorer. Disagreement requires refresh. No frontend gap becomes an allocation target.

Backend regions appear in the existing explorer and map. Region nodes retain stable object instances while discovery is current. Map/explorer selection updates the same node and inspector. Partitions retain real labels, filesystems, letters and paths. Targets provide native buttons, focus indicators, tab navigation, arrow/Home/End focus movement, Enter/Space selection, tooltips, accessible names and context menus. Tiny partitions have 36-point minimum widths; regions and proposals use 48 points. Dense maps scroll horizontally. Visual widths never determine geometry.

The page retains compact navigation and inspector breakpoints. Explorer names/types wrap, technical values support copying, and opening an editor brings it into view. Tooltips and the inspector retain full information for small targets.

## Creation preview

1. Select a disk and inspect its backend layout.
2. Select eligible unallocated space and choose Create partition preview.
3. Explicitly designate the exact identity through Session test media. The native dialog displays VPD identity, format and serial, defaults to Cancel and verifies unchanged discovery when confirmation returns.
4. Enter MiB/GiB size and an optional exact start in bytes. Blank start delegates placement to Rust. Only Microsoft basic-data GPT is exposed; filesystem/formatting controls are absent.
5. Request Preview or pause after editing for automatic preview. Reset restores 1 MiB and backend-selected start. Cancel closes the editor and invalidates pending results.

The editor distinguishes geometric, aligned and safely allocatable capacity. Conversion uses unsigned integers and `BigInteger` arithmetic for exact decimal fractions. Overflow, signs, exponent notation, grouping and fractional bytes are rejected. Inputs are bounded to 128 characters. Sizes are never rounded. Rust remains authoritative for minimum size, alignment, placement and safety.

Typed requests preserve `expected_identity`, opaque `expected_layout`, explicit `test_disk`, region identity, alignment and exact size/start. Edits debounce for 400 ms and wait for existing native work. The busy lease permits at most one active planning call.

Validated results show exact start, exclusive end, size, alignment and remaining geometric/aligned capacity. Separate Before and After maps project the approved extent. Proposed and remaining segments are labelled preview and cannot become allocation targets. Inventory is never edited. A preview neither reserves storage nor authorizes execution.

## Deletion preview

Selecting a partition and choosing Deletion preview opens inspection, including protected targets identifiable by the backend. The editor shows disk identity, partition GUID/number, exact size/type, associated volumes, mount paths, eligibility, restrictions and the backend data-loss warning.

Only a backend-eligible partition on designated media can submit a typed request. Identity, number, offset, size and GPT type come from discovery. A valid result replaces that extent with a labelled unallocated-after-deletion segment in a separate proposal map. This projection does not calculate future allocation eligibility or coalesce new allocation targets. Protected/rejected targets never receive an eligible proposal.

## Lifecycle and safety

Persistent identity and snapshots remain separate. Snapshots are opaque strings, never parsed or regenerated. Designation is an in-memory identity for the selected disk/session, without a saved allowlist. Another disk, refresh, leaving the page, stale/disconnected discovery and safety failures revoke it. Another partition/region retains the same disk's designation but discards the proposal/editor.

Planning generations invalidate late work. Input changes, Reset, Cancel, target changes and refresh clear proposals immediately. Responses must match generation, identity, snapshot and request parameters. Valid results must project onto the displayed layout. Failed validation cannot retain eligibility, even if a rejected response includes a preview payload.

Refresh invalidates before inventory retrieval. During a busy native call it invalidates immediately and queues retrieval for completion. The existing 15-second monitor uses this path. Windows removal/layout changes are detected by polling, with no new device-event subscription. Periodic refresh expires proposals/designations even when storage is unchanged. Each backend preview independently rediscovers the device.

FFI calls cannot be forcibly interrupted. Cancellation discards results while retaining the busy lease until return. Debounce work is cancelled/disposed on invalidation. Native work stays off the UI thread through the existing service.

Messages distinguish technical validity, protected/unverified targets, unsupported operations, disconnected devices, stale identity/layout, invalid input and discovery/permission failures. Backend codes and native error numbers remain visible. Unknown evidence never becomes affirmative eligibility.

All Milestone 6A Rust restrictions remain enforced: system/boot/protected targets, internal or unsuitable removable media, unsupported tables, incomplete safety evidence, collisions, changed snapshots and unsupported operations. Designation overrides none of them. Existing volume operations retain their independent confirmation/validation path.

## macOS presentation

SwiftUI places the physical map below capacity information. Map, partition list and sidebar share `DiskStore.selection`; store helpers select a partition or parent disk. Arrow commands move physical selection. APFS volumes highlight their store without adding physical segments.

Details show disk number, connection, scheme, partition count and honest read-only status. The grouped inspector adds exact capacity, reported bounds with exclusive end, parent information and protection explanations. Missing information stays unavailable. Adaptive inspector presentation, semantic colours and existing mount/unmount/eject/rename remain intact.

The workspace queries planning support through the existing Swift FFI service and stores one shared status. Selection, disk locator/identity, inventory revision, cancellation and generation guard responses. Selection/refresh/removal clear prior status; late results cannot overwrite another selection. Detail and inspector share a concise unsupported explanation and expandable backend details.

macOS never fabricates allocatable regions or advertises creation/deletion controls. APFS shared capacity remains distinct from physical unallocated capacity. macOS partition planning stays unsupported.

## Tests and validation

New Windows tests cover exact conversion beyond floating-point precision, fractional-byte rejection, overflow, typed create/delete requests, selection, designation mismatch, opaque snapshots, separate projections, protected/unsupported targets, backend errors, invalid input, refresh/removal, deferred refresh, stale responses, cancellation, busy exclusion, inventory disagreement and debounce. Fake services fail if planning attempts volume execution.

Swift tests distinguish pending/failed/unsupported states and verify bounds, unknown metadata and protection. Native tests cover shared selection, refresh/removal invalidation and late response rejection. Existing operation tests retain confirmation, capability-change, failure and refresh coverage with injected services. Rust adds optional hardware-metadata tests. Existing planning, malformed-request, stale identity/layout and no-execution FFI tests pass.

Validation on this Apple Silicon Mac:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --workspace --all-targets --locked` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `cargo test --workspace --locked` | 106 passed: 66 core unit, 31 planning integration, 9 FFI. |
| Windows x64 Rust cross-target Clippy, all targets, warnings denied | Passed; compilation checking only. |
| Portable .NET Debug/Release with CI analyzers and warnings denied | 88 passed each; native integration excluded. |
| Swift package with warnings denied | 19 tests passed, including parameterized cases. |
| SwiftLint 0.65.1, strict repository rules | Zero violations in 24 files. |
| Native Xcode Debug and `DiskvioTests` | Passed, 25 tests. |
| Xcode universal Release, compiler warnings denied | Passed; `lipo -archs` confirms `x86_64 arm64`. |
| WinUI XAML XML parsing and `git diff --check` | Passed; XML parsing is not XAML compilation. |

Rust checks ran through `python3 scripts/check.py all --universal`; Swift initially stopped at sandbox cache restrictions. `python3 scripts/check.py macos --universal` passed with required cache/test-service access after fixing a Swift 6 test compilation error. Existing nonfatal AppIntents metadata warnings and system shortcut-service diagnostics appeared during Xcode validation.

Live read-only inspection confirmed the macOS light-appearance workspace, exact-capacity inspector, unsupported explanation and revised map position. Computer-use selection was interrupted by an application-change guard. Dark appearance, compact/enlarged-text layouts and VoiceOver are not certified by this session. No real disk mutation, test-media designation or volume management action was performed.

## Limits and Milestone 6C

Windows is unavailable on this host. Visual Studio 2026 Debug/Release WinUI builds, XAML type compilation, MSIX packaging, rendering/accessibility and real GPT/USB discovery remain required. Portable .NET tests and Rust cross-target checks do not establish them. Run `python scripts/check.py windows --native-integration` on Windows, then exercise preview workflows with expendable eligible external test media without execution.

Before Milestone 6C, verify Windows light/dark/high-contrast, smaller windows, long labels, many/tiny partitions, keyboard/context menus, Narrator, disconnects and external layout changes during requests. Keep conservative pool, encryption, Windows-directory and identity requirements. Native device notifications can replace polling in a separately tested improvement.

Milestone 6C has typed configuration, explicit designation, lifecycle tests and separate proposal maps to build on. Execution needs its own native implementation, handle-bound immediate identity/layout/safety revalidation, destructive confirmation, failure/recovery design and Windows hardware testing. No preview authorizes disk modification. macOS allocation support remains a separate backend milestone.
