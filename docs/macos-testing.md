# Xcode and hardware testing

## Build and test

1. Open `macos/Diskvio/Diskvio.xcodeproj` in Xcode and select the **Diskvio** scheme with **My Mac** as the destination.
2. Ensure Cargo is installed at `~/.cargo/bin` and the Rust target matching your Mac is installed. The existing Build Rust FFI phase builds and links the static library automatically.
3. Use **Product → Build** (Command-B), then **Product → Run** (Command-R). Inspection does not need an administrator account or sudo.
4. Use **Product → Test** (Command-U) to run the Swift tests. The explorer unit tests use fixtures and never manage real disks.

To reproduce command-line validation:

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
xcodebuild -project macos/Diskvio/Diskvio.xcodeproj -scheme Diskvio \
  -configuration Debug -destination 'platform=macOS' \
  -derivedDataPath /private/tmp/diskvio-derived build CODE_SIGNING_ALLOWED=NO
xcodebuild -project macos/Diskvio/Diskvio.xcodeproj -scheme Diskvio \
  -configuration Debug -destination 'platform=macOS' \
  -derivedDataPath /private/tmp/diskvio-derived test \
  -only-testing:DiskvioTests CODE_SIGNING_ALLOWED=NO
```

In restricted agent environments, DiskManagement access and Swift macro plugins may be blocked by the agent's sandbox. These commands were validated outside that agent sandbox; this does not imply administrative elevation or a change to the application's sandbox setting.

## Physical device checks

Use an external USB disk whose files can be closed for this test. Do not use an internal or boot/system disk for management checks.

1. Launch without the USB connected, then connect it. Confirm a physical disk appears automatically and expands into its actual partitions. Compare identifiers, scheme and capacity with Disk Utility or `diskutil list -plist physical`.
2. For APFS, verify the container is beneath its physical store and the volumes are beneath the container. Volume capacity is shared; volume sizes must not become extra segments in the physical partition map. Inspect quota/usage, UUIDs, roles, encryption, and mounted snapshot properties if applicable.
3. Expand the native disclosure triangles to reach a partition, APFS container, and volume. Select a partition or volume and press Command-R. The same selection and expanded branches should remain. Resize the window, drag splitters, and check the app in both macOS light and dark appearances. At narrow workspace widths, the inspector button opens a properties popover; at wider widths it toggles the resizable inspector pane. The shortcut is Option-Command-I.
4. Select a mounted supported external volume and choose **Unmount**. Confirm success or a useful busy/permission error. If successful, check the mount point disappears; select **Mount** and check it returns.
5. Select the physical external disk and choose **Eject**. Confirm the hierarchy disappears and selection safely changes. Reconnect the disk and verify it returns without restarting Diskvio.
6. Disconnect the external disk while its volume is selected. Confirm no stale detail/actions remain and the window stays responsive. Repeat with a refresh in progress to exercise stale-snapshot handling.
7. Keep a file open on the external volume and try to unmount. Diskvio must report macOS's result and never force-unmount or silently elevate privileges.
8. Verify internal disks and their descendants have no management actions. A locked APFS volume must remain unavailable for Mount until you unlock it using Disk Utility and refresh.

## Validation performed

On 8 October 2026, the Rust formatting/check/Clippy/test gates and the Xcode Debug build passed after each milestone. The final workspace run passed 19 Rust tests, and Clippy also passed with `--all-targets`. Eight Swift unit tests passed for the legacy envelope, richer topology, error responses, unknown/large capacities, UUID selection, immediate removal, and coalescing/initial notification suppression. Xcode reported that App Intents metadata extraction was skipped because the app does not depend on AppIntents.

Actual read-only discovery and app launch were tested on this Mac: one physical disk, three physical partitions, three physical-backed APFS containers, and twelve APFS volumes, including a mounted system snapshot. The UI was exercised for physical-disk and APFS-volume selection, selection preservation after Refresh, and native window layout. An initially observed split-view layout crash was fixed and the app relaunched successfully.

Physical USB attach/remove and successful mount/unmount/eject were **not** exercised in this session. Notification state transitions and safety policies were tested with fixtures; these do not constitute hardware-operation verification. Dark appearance and distribution signing/notarization were not runtime-verified. Windows discovery was preserved in source but was not built or run on Windows here.

The subsequent UI refinement passed the Xcode build and eleven Swift unit tests, including physical partition geometry for leading gaps, unknown sizes, out-of-bounds segments and overflowing offsets. Real-device hierarchy expansion, minimum/maximum sidebar widths, the smallest native window layout, native zoom, keyboard row selection, Refresh preservation, graphical partition selection and the narrow-workspace inspector popover were visually exercised. These visual checks preceded the final compact-caption and selected-foreground changes; those changes compiled and passed the tests, but final screenshot capture was blocked when native automation failed globally with “Sky Computer Use native pipe startup failed.” An independent source review found no material implementation defect, but the formal visual finish review remains awaiting recapture. Full VoiceOver traversal, unusual external-device names and disks with many physical partitions still need runtime coverage.
