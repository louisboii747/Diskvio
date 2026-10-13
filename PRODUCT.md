# Diskvio

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

Native macOS SwiftUI and Windows WinUI 3 frontends over a shared Rust workspace. Windows supports preview-only partition planning; macOS reports planning as unsupported.

## Product Purpose

Inspect real connected physical disks and their partition/filesystem topology, then explicitly mount or unmount supported external volumes or eject removable/ejectable external disks. Reliability, correct disk information, native integration, and maintainability take priority over implementation speed.

## Operating Context

The macOS app builds through Xcode at `macos/Diskvio/Diskvio.xcodeproj`; Windows builds through the Visual Studio 2026 WinUI solution. `diskvio-core`, `diskvio-cli` and `diskvio-ffi` provide shared backend contracts through a stable C ABI. Preserve native controls, existing CLI behavior and supported volume operations on both platforms.

## Capabilities and Constraints

The hierarchy is Physical Disk → Partition → APFS Container → APFS Volume. Containers may have multiple physical stores; synthesized volumes and mounted snapshots must not be classified as physical partitions. Discovery must use structured platform data, expose unavailable fields honestly, and avoid administrator privileges for inspection.

The interface needs a sidebar, central detail, proportional physical partition map, inspector, loading/empty/error states, Refresh, context menus, human-readable capacity, and stable selection across refresh. Device appearance/removal should update the app automatically with properly cleaned-up native observers and coalesced scans.

Every management operation requires an explicit user action and a verified external target. Internal disks and protected system volumes are inspection-only. No formatting, partition deletion/resizing, raw disk writing, ISO flashing, automatic management actions, force-unmount, or silent elevation belong to these milestones. Unlocking encrypted disks and complete RAID/CoreStorage exploration remain outside the implementation.

## Brand Commitments

The name is Diskvio. The user pinned a native macOS appearance, system typography, SF Symbols, light/dark adaptation, clear information hierarchy, consistent spacing, proper resizing, and minimal visual clutter. No alternate visual identity is requested.

## Evidence on Hand

All displayed app data comes from Rust platform discovery. Rust tests and Swift decoding/state tests use explicit fixtures, never product demonstration data. Actual read-only discovery and app launch were tested on this Mac; physical USB management and Windows runtime validation remain unverified. See `docs/macos-testing.md` for exact validation scope and manual test steps.

## Product Principles

- Preserve real storage relationships and avoid misleading capacity accounting.
- Keep the Rust/Swift ownership boundary explicit and memory-safe.
- Use native macOS controls and platform mechanisms.
- Require explicit, verifiable targets for management operations.
- Report failures and validation limits honestly.

Audience segmentation, commercial positioning, licensing, and distribution beyond the existing non-sandboxed Xcode target are not specified by this milestone brief.
