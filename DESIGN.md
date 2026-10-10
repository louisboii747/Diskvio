---
name: Diskvio
description: Native macOS storage inspection with a restrained functional hierarchy.
spacing:
  compact: 2
  metadata: 3
  summary: 5
  inline: 8
  related: 10
  inset: 12
  section: 24
rounded:
  partition-map: "5px"
components:
  detail-content:
    padding: "{spacing.section}"
  notice:
    padding: "{spacing.inset}"
  partition-map:
    rounded: "{rounded.partition-map}"
    height: "64px"
  partition-legend-marker:
    size: "8px"
---

# Design System: Diskvio

## Overview

**Creative North Star: "Native macOS Utility"**

Diskvio uses familiar macOS controls to make storage relationships legible. Its identity comes from clear hierarchy, real metadata, consistent spacing, and restrained emphasis. System typography, SF Symbols, and native selection and focus treatments keep the interface at home on macOS.

The application is a working utility with concise labels, readable properties, and enough separation to distinguish devices, capacity summaries, and supporting explanations. Light and dark appearance follow platform semantics. This document records the implemented SwiftUI system; it does not certify untested runtime states.

**Key Characteristics:**

- Native SwiftUI navigation, lists, forms, toolbars, and context menus.
- Semantic text hierarchy and appearance-aware platform colors.
- Flat surfaces with structural dividers and restrained tonal emphasis.
- SF Symbols and text labels that explain storage relationships.
- Resizable panes and scrollable detail content.

## Colors

The palette uses macOS semantic surfaces and foregrounds, with system hues reserved for the physical partition map.

### Primary

Native controls own accent, selection, hover, focus, and disabled appearance. The source does not establish a custom brand accent; the AccentColor asset contains no color value.

### Secondary

**Partition categories** use native orange for system/EFI, green for recovery, grey for reserved/metadata, blue for data/APFS stores, and the native secondary-label tone for unknown types. Categories derive from reported role/content and protection metadata. Names and a legend accompany colours; selected segments receive a two-point accent border. Tinted fills retain semantic text foregrounds. Colour does not establish whether an operation is safe; backend capabilities do.

### Neutral

- **Default foreground:** native text and primary values.
- **Secondary foreground:** metadata, captions, explanations, and supporting symbols.
- **Tertiary foreground:** child-row disclosure chevrons.
- **Quaternary fill:** notices and restrained metadata badges.
- **Text background:** `NSColor.textBackgroundColor` behind the central detail view.

These dynamic native values cannot be represented faithfully by static CSS color tokens. Their exact SwiftUI/AppKit mappings live in `.impeccable/design.json`; no fixed hexadecimal substitutes or synthesized tonal ramps are normative.

**The Platform Appearance Rule.** Use the existing semantic platform colors and native control states so appearance follows macOS.

## Typography

**System Font:** SwiftUI's default macOS system typography. SF Symbols provide interface icons; there are no custom fonts or interface glyph assets. The application icon uses the supplied artwork in `assets/`.

### Hierarchy

- **Selection title:** `.title2` with `.semibold`, used for the selected device name.
- **Capacity value:** `.title3` with `.monospacedDigit()`, used for summary figures.
- **Section heading:** `.headline`, used for child lists and the physical partition map.
- **Body:** the inherited system text style, used for names, identifiers, and inspector values.
- **Supporting copy:** `.callout`, used for storage explanations, map labels, and status details.
- **Metadata label:** `.caption` with a secondary foreground, used for capacity labels, properties, row metadata, and update information.
- **Row emphasis:** `.medium` weight for child-device names and notice titles.

Semantic text styles remain semantic; this system does not replace them with inferred point sizes, line heights, tracking, or a fixed type-scale ratio. The selected-device symbol has an explicit icon size, documented separately from text roles in the sidecar. Capacity figures use tabular digits without switching the whole interface to a monospaced face.

**The Metadata Hierarchy Rule.** Keep names and values primary, and use secondary captions for their labels and context.

## Layout

The native split-view model separates navigation from the active working area. `ContentView` owns the `NavigationSplitView`, with `DeviceSidebar` as navigation and `DeviceWorkspaceView` as the working area. The workspace owns its toolbar, status regions, detail, and inspector. `WorkspaceLayout` holds the shared pane budget. The detail scrolls vertically, fills available width, and uses a leading alignment. The inspector uses a native grouped `Form`.

The spacing tokens above are unitless native point values. CSS-compatible dimensions in the frontmatter represent the same logical measurements for documentation previews; the sidecar records their SwiftUI point mappings. Use compact gaps for stacked metadata, the inline and related gaps for associated content, the inset for status and notice regions, and the section gap for central content and capacity groups. There is no universal eight-point grid in the source.

The implemented pane limits, minimum window size, and default window size are recorded in the sidecar as macOS layout metrics. The main content keeps its minimum width. When the workspace can accommodate both panes, an `HSplitView` presents the optional inspector inline; at narrower widths the same Inspector command opens properties in a popover. Capacity summaries and partition legends use adaptive grids so their text remains useful as the detail width changes.

**The Content Width Rule.** Preserve the central workspace's minimum width and move properties to the inspector popover when the inline pane budget does not fit.

Retain resize affordances, the inspector command, and scrollable detail content. The refinement was observed in the smallest allowed native window, at both sidebar limits, in a maximized window, and with the narrow inspector popover. Final compact-caption and selected-foreground changes compiled and passed tests; their screenshot recapture was blocked by native automation failure. Enlarged-text, dark-appearance, and VoiceOver runtime validation remain outstanding; observed resizing does not certify all accessibility states.

## Elevation & Depth

Custom surfaces are flat. The app defines no custom shadow or blur tokens. Native split-view separators, `Divider`, grouped-form containment, and semantic backgrounds establish depth. Notices use a quaternary fill; the central detail uses the platform text background. System controls retain their own platform appearance.

## Shapes

Native controls own their corners and borders. The custom physical map clips its rectangular segments to the documented partition-map radius. Its legend uses circular markers, and clickable child rows use rectangular content shapes. The map's radius is a signature-component measurement, not a global corner rule for buttons, forms, or notices.

## Components

### Buttons and Toolbar

Commands use native `Button` and `Label`, with SF Symbols, descriptive help, and native disabled states. Refresh has the Command-R shortcut. Inspector has Command-Option-I and toggles the inline pane or opens the narrow-workspace popover. The toolbar groups supported management commands in an Actions menu only when the selected node has real available actions. `DeviceActionButtons` supplies both that menu and `DeviceContextMenu`; the context menu also provides Finder, identifier-copy, and Refresh commands when applicable. Child-device rows and map segments use the existing plain button style; preserve native keyboard and focus behavior.

### Navigation

The sidebar is a native sidebar-style selectable `List` with an `OutlineGroup` over the real device hierarchy. Native outline controls own disclosure gutters and indentation. Each bounded row combines its device symbol, a single-line tail-truncated name, and caption metadata. The caption shows kind and capacity when both fit, then retains capacity alone when the row is narrower. Selected symbols and captions inherit the native selection foreground; unselected supporting content uses the secondary foreground. The tooltip and combined accessibility label retain the full name, kind, identifier, and capacity. Hierarchy represents actual physical disk, partition, APFS container, and volume relationships.

### Capacity Summary

A compact vertical pair places a secondary caption above a title-style capacity value with tabular digits. The pairs flow through an adaptive grid in `DeviceDetailView`. Decimal byte formatting comes from `ByteCountFormatter`; absent capacities read "Not available". APFS volume totals are explicitly labeled "Shared capacity" or "Capacity limit", and available space is labeled "Available to volume". Retain the supporting explanation that APFS volumes share container free space.

### Inspector Properties

A native grouped form contains device and volume/storage sections. Each property stacks a secondary caption over its selectable value. Long values can wrap vertically. Each row has a combined accessibility label and value. Missing metadata remains explicit rather than disappearing or receiving a fabricated value.

### Notices and Loading States

Notices pair a secondary SF Symbol with a medium-weight title and secondary caption detail on a quaternary background. Error and success meaning comes from the title, symbol, and message, rather than a custom status palette. Native progress controls and content-unavailable views supply loading, discovery failure, and empty states. These native states have no bespoke shadow, radius, or animation tokens.

### Physical Partition Map

The 64-point track compares reported physical partition capacities with a 48-point minimum target and four-point spacing. Remaining width is capacity-weighted; dense layouts scroll horizontally. The geometry is explicitly schematic and does not plot gaps or claim unallocated regions. Unknown capacities remain unknown, and APFS volumes do not receive independent physical segments.

A synchronized partition list shows names, formats, mount paths/identifiers and exact capacities. Tooltips, accessibility labels, keyboard-focusable buttons and context menus keep tiny partitions inspectable. Selection uses an accent border and list highlight; an APFS volume highlights its physical store. `PartitionPresentation` supplies consistent names/categories, while `PartitionMapLayout` supplies independently tested widths.

**The Partition Capacity Rule.** Compare reported physical capacities with documented minimum visual widths, retain exact values in text, and keep APFS sharing explicit. Do not infer allocatable space from gaps.

### Management and Validation Scope

Rename uses a native sheet with label entry, backend validation preview and explicit confirmation. Every mount/unmount/eject also requires confirmation. Context actions and the selected-item management section use real backend capabilities and explanations. System/boot/recovery, external/USB, read-only and encryption badges appear only from reported metadata.

Milestone 5 portable Swift models and Rust fixtures were tested on Windows. Historical native layout observations earlier in this document predate these changes. Current native Xcode build, light/dark rendering, enlarged text, VoiceOver and external-volume rename need a Mac; see `docs/milestone-5-macos-refinement.md`.

## Do's and Don'ts

### Do:

- **Do** use native macOS SwiftUI controls, SF Symbols, system typography, and semantic colors.
- **Do** preserve primary values with secondary labels and the implemented spacing roles.
- **Do** use tabular digits for capacity values and retain explicit shared-capacity language for APFS volumes.
- **Do** pair partition hues with names, identifiers, and capacities.
- **Do** retain resizable panes, scrollable detail, the inspector toggle, and explicit unavailable metadata.

### Don't:

- **Don't** replace appearance-aware native colors or semantic text styles with guessed fixed values.
- **Don't** use colour alone to communicate category, selection or operation availability.
- **Don't** render shared APFS volume capacity as an independent physical partition.
- **Don't** generalize the map radius into a custom style for native controls.
- **Don't** present source-level layout or accessibility affordances as runtime validation.
