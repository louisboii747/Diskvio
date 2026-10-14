---
name: Diskvio
description: Native SwiftUI and WinUI storage inspection with truthful metadata.
colors:
  windows-ntfs-light: "#2463AB"
  windows-ntfs-dark: "#8BBBF0"
  windows-apfs-light: "#7653AD"
  windows-apfs-dark: "#C2A2EF"
  windows-fat-light: "#247442"
  windows-fat-dark: "#8BD1A3"
  windows-exfat-light: "#147D83"
  windows-exfat-dark: "#83CED0"
  windows-ext-light: "#956519"
  windows-ext-dark: "#E5BF7A"
  windows-system-light: "#986000"
  windows-system-dark: "#E8B45D"
  windows-recovery-light: "#247442"
  windows-recovery-dark: "#73C991"
  windows-reserved-light: "#757575"
  windows-reserved-dark: "#ADADAD"
  windows-data-light: "#2463AB"
  windows-data-dark: "#74B5F4"
  windows-unknown-light: "#595959"
  windows-unknown-dark: "#858585"
  windows-unmapped-light: "#303030"
  windows-unmapped-dark: "#121212"
typography:
  windows-title:
    fontSize: "24px"
    fontWeight: 600
  windows-summary:
    fontSize: "20px"
    fontWeight: 600
  windows-section:
    fontSize: "16px"
    fontWeight: 600
  windows-property:
    fontSize: "14px"
  windows-map-caption:
    fontSize: "12px"
rounded:
  marker: "2px"
  windows-gap: "3px"
  badge: "4px"
  macos-surface: "6px"
spacing:
  tight: "4px"
  compact: "6px"
  inline: "8px"
  row: "10px"
  inset: "12px"
  related: "14px"
  heading: "16px"
  property: "20px"
  section: "24px"
components:
  macos-search:
    rounded: "{rounded.macos-surface}"
    padding: "7px"
  macos-badge:
    rounded: "{rounded.badge}"
    padding: "4px 8px"
  macos-partition:
    rounded: "{rounded.macos-surface}"
    padding: "{spacing.row}"
    height: "96px"
  macos-partition-row:
    rounded: "{rounded.macos-surface}"
    padding: "{spacing.row}"
  windows-partition:
    padding: "{spacing.tight}"
    height: "102px"
  windows-explorer-row:
    rounded: "{rounded.badge}"
  detail-content:
    padding: "{spacing.section}"
---

# Design System: Diskvio

## Overview

**Creative North Star: "Native Storage Utility"**

Diskvio makes real disks, partitions and volumes legible through native platform controls. Its identity is the Diskvio name and supplied app artwork, system typography, clear information hierarchy and restrained emphasis. macOS uses SwiftUI and SF Symbols; Windows uses WinUI 3, Fluent controls and native icon glyphs.

The working surface is dense enough for inspection and calm enough to compare metadata. Native selection, focus, menus and semantic surfaces carry interaction. Filesystem color helps identify partitions, while exact text and backend metadata remain authoritative. This document records implemented design; runtime validation is scoped in [the redesign report](docs/native-ui-redesign.md).

**Key Characteristics:**

- Native controls and semantic light/dark appearance on both platforms.
- Grouped device browsing with search, filters and visible parent-disk association.
- Flat detail surfaces, structural dividers and a separate information inspector.
- Filesystem-aware partition maps synchronized with selectable rows.
- Explicit unavailable metadata, loading, errors and device-disconnection feedback.

## Colors

### Primary

There is no custom brand accent. SwiftUI uses `Color.accentColor`; WinUI uses `SystemControlHighlightAccentBrush`. Native controls own ordinary hover, focus, pressed and disabled treatments. The macOS AccentColor asset does not establish a fixed color.

### Secondary

Filesystem colors are categorical aids. On macOS, NTFS and other data use `.blue`, APFS uses `.purple`, FAT and recovery use `.green`, exFAT uses `.teal`, ext and system use `.orange`, and metadata or unknown types use `.gray`. The map uses a solid category strip, a low-opacity category fill, a legend and text. Reported filesystem metadata determines the tone before the reported partition category fallback.

Windows category values in the frontmatter come directly from the Light and Default dictionaries in `windows/Diskvio.Windows/App.xaml`. Each key maps to its corresponding `PartitionNTFSBrush`, `PartitionAPFSBrush`, `PartitionFATBrush`, `PartitionExFATBrush`, `PartitionExtBrush`, `PartitionSystemBrush`, `PartitionRecoveryBrush`, `PartitionReservedBrush`, `PartitionDataBrush`, `PartitionUnknownBrush` or `PartitionUnmappedBrush`. HighContrast maps category brushes to `SystemColorWindowTextColor` and unmapped space to `SystemColorWindowColor`. Ambiguous filesystem combinations fall back to the reported partition category.

### Neutral

macOS uses inherited primary text, `.secondary`, `.tertiary`, `.quaternary` and `NSColor.textBackgroundColor`. WinUI uses inherited foregrounds plus `TextFillColorSecondaryBrush`, `SubtleFillColorSecondaryBrush`, `DividerStrokeColorDefaultBrush` and `ControlStrokeColorDefaultBrush`. Mica is requested only when the platform supports it. No fixed CSS substitutes or synthesized tonal ramps are normative for these dynamic native values; the sidecar records their source mappings.

**The Platform Appearance Rule.** Use semantic native values for surfaces, text and control states; keep explicit category colors in their existing platform theme resources.

## Typography

SwiftUI keeps semantic system styles: the selection title uses `.title2.weight(.semibold)`, capacities use `.title3.weight(.medium)` with `.monospacedDigit()`, section titles use `.headline`, supporting text uses `.callout`, and metadata uses `.caption` or `.caption2`. Body text inherits the platform style. The overview symbol is an icon measurement, not a text style.

WinUI keeps its native font family and control typography. Explicit title, summary, section, property and map-caption measurements appear in the frontmatter; caption labels use `CaptionTextBlockStyle`. Names are medium or semibold where needed. Long inspector values wrap and support selection or copying. There are no custom font files, fixed line-height scales or tracking tokens.

**The Metadata Hierarchy Rule.** Keep names and values primary, and use secondary captions for their labels and context.

## Layout

Dimensions in the frontmatter use CSS-compatible notation for tooling; in the app they are SwiftUI points or WinUI effective pixels. The spacing scale captures recurring values, not a universal grid.

macOS uses `NavigationSplitView`, a sidebar-style list, a scrolling detail and an optional inspector in `HSplitView`. The window minimum is 900 by 600, with a default of 1280 by 780. Sidebar minimum, ideal and maximum widths are 240, 280 and 400. Central content retains a minimum width of 440. Inspector widths are 240, 280 and 340. It fits inline when the working area is at least 760 wide; otherwise the Inspector command opens the same content in a popover. Capacity summaries and legends adapt their columns to available width.

Windows starts at 1280 by 780. Its sidebar starts at 264 and resizes between 220 and 380, additionally constrained by the remaining workspace. An eight-unit Thumb supports dragging and Left/Right keyboard resizing in sixteen-unit steps. Below a page width of 900, navigation moves above the detail in a 200-unit row. The 280-unit inspector sits beside content when page width minus the current sidebar width is at least 920; otherwise it follows content below a divider. The partitions-and-volumes table retains a 720-unit minimum and scrolls horizontally.

**The Content Width Rule.** Preserve readable content and inspectable targets through native pane resizing, adaptive inspector placement and scrolling.

## Elevation & Depth

Custom working surfaces are flat and define no shadow tokens. Native split-view separators, grouped forms, theme dividers and semantic backgrounds establish containment. macOS notices and badges use quaternary fill. Windows retains native control depth and an optional Mica window backdrop. Do not apply custom shadows to turn overview groups into decorative cards.

## Shapes

Native controls retain their native corners and borders. Custom macOS search, partition segments and selected rows share the documented gently curved surface radius; badges use the smaller radius. Map category markers are circles or narrow rounded bars. Windows explorer items use the badge radius, category strips use the marker radius, and noninteractive gap fills use the gap radius. These measurements do not redefine native button, menu, dialog or form shapes.

## Components

### Commands and Inputs

Native toolbars and command bars expose Refresh, inspector visibility and existing supported management actions. macOS uses Command-R and Command-Option-I; Windows uses Ctrl-R or F5 and Ctrl-I. Search uses a SwiftUI text field with a clear button or a WinUI `AutoSuggestBox`. Device filtering and partition sorting use native pickers or combo boxes. Capability checks and existing confirmations determine whether management controls can act.

### Device Browser

Both platforms group physical disks as Internal, External, Removable or Other devices from reported metadata. Filters cover all, internal, external and removable devices. macOS retains its real outline hierarchy when search is empty. Windows normally lists disks and exposes their partitions and volumes in the working area, with an additional unattached-volumes expander.

Active search directly lists matching disks, partitions and volumes, including each child result's parent-disk association. Matching covers names, identifiers, reported filesystems, mount locations, model, manufacturer and connection. Search and filters do not invent device metadata or change the selected operation target. Empty results explain the absence of matches. Full names and identifiers remain available through tooltips and accessibility labels.

### Overview and Capacity

The selected-device overview combines a native icon, name, identifier, reported model and connection, partition scheme, status and capacities. macOS adds reported protection badges and a usage indicator only when relevant values exist. Exact-byte tooltips accompany formatted capacity values. Missing values remain explicitly unavailable. APFS volumes retain shared-capacity or quota labels and an explanation that shared free capacity is not unallocated disk space.

### Information Inspector

macOS uses a grouped `Form` with General, Storage, Connection, Protection and Advanced sections. Windows groups selectable properties as General, Storage, Connection, Protection & actions and Advanced. Copy actions cover the selected identifier and supported property values. Technical identities, exact capacities and backend limitations remain available without crowding the overview.

### Partition Map and Rows

macOS segments have a 48-point minimum width, four-point spacing and a 96-point button height in a horizontally scrolling track. Remaining width is distributed by reported capacity. Labels expand at 110 points; narrow segments retain a number. Gaps are not plotted. APFS volumes highlight their physical store rather than receiving fictitious physical segments. A legend and sortable partition list retain full names, format, identifier, exact-capacity tooltips and available usage information.

Windows minimum allocation widths are 36 for reported partitions, 48 for backend regions or proposals and four for unmapped gaps. Interactive buttons reserve three units within their allocation; exact byte values remain in tooltips. Segment buttons are 102 high in a 122-high scroll control, with expanded labels at 90 units. Valid reported offsets allow unmapped metadata gaps; missing or inconsistent positioning falls back to a capacity comparison. Only Rust-reported regions can be presented as allocatable. Existing proposed layouts remain separate and read-only.

Selection synchronizes the map, working rows and inspector; sorting changes list order without changing the physical map or selected identity. Selected segments add an accent border and a checkmark; macOS rows also add an accent-tinted background. macOS Left/Right movement tracks the focused segment, moves focus and selection, and scrolls the destination into view. Windows Left/Right/Home/End move native button focus; Enter or Space selects. Context menus use the selected real device and existing capabilities.

macOS selection animation uses `.easeOut(duration: 0.16)` only when Reduce Motion is off. Windows adds no custom animation timeline and retains default native control motion.

**The Partition Capacity Rule.** Keep minimum visual widths explicit, preserve exact byte values, and distinguish physical partitions, shared filesystem capacity, unmapped metadata and backend allocation regions.

### Feedback and Partition Previews

Native progress controls distinguish initial discovery from refresh and active operations. Errors retain their message and retry path; warnings expose incomplete data. Dismissible completion and disconnection notices preserve context when inventory changes. Selection reconciles against refreshed backend identities.

Windows retains existing partition previews in the `Partition planning · Preview only` expander, which opens when entering the planning editor. macOS shows the backend's unsupported planning state in the inspector. No formatting, erase, resize, raw-write or partition-commit control is introduced by this redesign. Rust safety policy, stable JSON/C ABI boundaries and explicit supported-operation confirmation remain authoritative.

## Do's and Don'ts

### Do:

- **Do** use native system typography, semantic foregrounds and platform control states.
- **Do** preserve Diskvio branding and the supplied app artwork.
- **Do** show parent relationships, exact capacities and unavailable metadata honestly.
- **Do** keep map, browser, rows and inspector selection consistent.
- **Do** preserve native resizing, scrolling, keyboard access and backend capability checks.

### Don't:

- **Don't** infer free space or allocation authorization from visual gaps or colors.
- **Don't** present APFS shared capacity as an independent physical region.
- **Don't** replace semantic native values with guessed fixed colors or custom control skins.
- **Don't** add decorative cards, custom shadows or an alternate visual identity.
- **Don't** treat static checks or portable tests as native runtime or whole-product visual approval.
