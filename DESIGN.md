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
    height: "38px"
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

**Partition hues** are SwiftUI system blue, teal, orange, purple, pink, and indigo, assigned in that order and repeated by partition index. The legend and segment share the same hue. These colors distinguish physical partitions; they do not encode filesystem type, risk, or operation success. Selected segments use full opacity, while other segments use the implemented reduced opacity recorded in the sidecar.

### Neutral

- **Default foreground:** native text and primary values.
- **Secondary foreground:** metadata, captions, explanations, and supporting symbols.
- **Tertiary foreground:** child-row disclosure chevrons.
- **Quaternary fill:** notices and the partition-map base showing gaps and metadata.
- **Text background:** `NSColor.textBackgroundColor` behind the central detail view.

These dynamic native values cannot be represented faithfully by static CSS color tokens. Their exact SwiftUI/AppKit mappings live in `.impeccable/design.json`; no fixed hexadecimal substitutes or synthesized tonal ramps are normative.

**The Platform Appearance Rule.** Use the existing semantic platform colors and native control states so appearance follows macOS.

## Typography

**System Font:** SwiftUI's default macOS system typography. SF Symbols provide interface icons; there are no custom font or icon assets.

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

The native split-view model separates navigation from the active working area. `NavigationSplitView` provides the sidebar, and an `HSplitView` separates the central detail from the optional properties inspector. The detail scrolls vertically, fills available width, and uses a leading alignment. The inspector uses a native grouped `Form`.

The spacing tokens above are unitless native point values. CSS-compatible dimensions in the frontmatter represent the same logical measurements for documentation previews; the sidecar records their SwiftUI point mappings. Use compact gaps for stacked metadata, the inline and related gaps for associated content, the inset for status and notice regions, and the section gap for central content and capacity groups. There is no universal eight-point grid in the source.

The implemented pane limits, minimum window size, and default window size are recorded in the sidecar as macOS layout metrics, not web breakpoints. Retain resize affordances, the inspector toggle, and scrollable detail content. Minimum-width and enlarged-text behavior still require runtime validation; these measurements are implementation evidence rather than an accessibility guarantee.

## Elevation & Depth

Custom surfaces are flat. The app defines no custom shadow or blur tokens. Native split-view separators, `Divider`, grouped-form containment, and semantic backgrounds establish depth. Notices use a quaternary fill; the central detail uses the platform text background. System controls retain their own platform appearance.

## Shapes

Native controls own their corners and borders. The custom physical map clips its rectangular segments to the documented partition-map radius. Its legend uses circular markers, and clickable child rows use rectangular content shapes. The map's radius is a signature-component measurement, not a global corner rule for buttons, forms, or notices.

## Components

### Buttons and Toolbar

Commands use native `Button` and `Label`, with SF Symbols, descriptive help, and native disabled states. Refresh has the Command-R shortcut. The toolbar also exposes the inspector toggle and available device actions. Child-device rows and map segments use the existing plain button style; preserve native keyboard and focus behavior. Context menus share device actions and provide contextual Finder and identifier commands.

### Navigation

The sidebar is a native sidebar-style selectable `List` with nested `DisclosureGroup` branches. Each row combines its device symbol, a single-line name, and secondary caption metadata. Native selection identifies the active node. The row tooltip includes the name and identifier. Hierarchy represents actual physical disk, partition, APFS container, and volume relationships.

### Capacity Summary

A compact vertical pair places a secondary caption above a title-style capacity value with tabular digits. Decimal byte formatting comes from `ByteCountFormatter`; absent capacities read "Not available". APFS volume totals are explicitly labeled "Shared capacity" or "Capacity limit", and available space is labeled "Available to volume". Retain the supporting explanation that APFS volumes share container free space.

### Inspector Properties

A native grouped form contains device and volume/storage sections. Each property stacks a secondary caption over its selectable value. Long values can wrap vertically. Each row has a combined accessibility label and value. Missing metadata remains explicit rather than disappearing or receiving a fabricated value.

### Notices and Loading States

Notices pair a secondary SF Symbol with a medium-weight title and secondary caption detail on a quaternary background. Error and success meaning comes from the title, symbol, and message, rather than a custom status palette. Native progress controls and content-unavailable views supply loading, discovery failure, and empty states. These native states have no bespoke shadow, radius, or animation tokens.

### Physical Partition Map

The map's fixed-height track depicts physical partition capacity and offset relative to its physical disk. Segments and legend entries are plain buttons selecting the same partition. Legend names, identifiers, capacity values, tooltips, and accessibility labels accompany color. Neutral track space represents gaps and partition-map metadata. APFS volumes do not receive independent physical-partition segments.

**The Physical Scale Rule.** Map geometry follows real physical partition bytes and offsets; container and volume sharing remain explicit in text.

## Do's and Don'ts

### Do:

- **Do** use native macOS SwiftUI controls, SF Symbols, system typography, and semantic colors.
- **Do** preserve primary values with secondary labels and the implemented spacing roles.
- **Do** use tabular digits for capacity values and retain explicit shared-capacity language for APFS volumes.
- **Do** pair partition hues with names, identifiers, and capacities.
- **Do** retain resizable panes, scrollable detail, the inspector toggle, and explicit unavailable metadata.

### Don't:

- **Don't** replace appearance-aware native colors or semantic text styles with guessed fixed values.
- **Don't** assign status or filesystem meaning to the partition color sequence.
- **Don't** render shared APFS volume capacity as an independent physical partition.
- **Don't** generalize the map radius into a custom style for native controls.
- **Don't** present source-level layout or accessibility affordances as runtime validation.
