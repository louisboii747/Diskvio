import SwiftUI

struct PartitionMapView: View {
    let disk: PhysicalDisk
    let selectedIdentifier: String
    @ObservedObject var store: DiskStore
    let select: (String) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var sort = PartitionSort.position
    @FocusState private var focusedSegment: String?
    private var layout: PartitionMapLayout { PartitionMapLayout(disk: disk) }
    private var sortedSegments: [PartitionMapSegment] {
        layout.segments.sorted { first, second in
            switch sort {
            case .position: first.index < second.index
            case .name: first.title.localizedStandardCompare(second.title) == .orderedAscending
            case .capacity: (first.partition.device.sizeBytes ?? 0) > (second.partition.device.sizeBytes ?? 0)
            }
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text("Partition layout").font(.headline)
                Spacer()
                Text("\(disk.partitions.count) \(disk.partitions.count == 1 ? "partition" : "partitions")")
                    .font(.caption).foregroundStyle(.secondary)
            }
            if layout.segments.isEmpty {
                Label("No physical partitions were reported for this disk.", systemImage: "rectangle.dashed")
                    .foregroundStyle(.secondary).padding(.vertical, 16)
            } else {
                GeometryReader { geometry in
                    let widths = layout.visualWidths(availableWidth: Double(geometry.size.width))
                    ScrollViewReader { proxy in
                        ScrollView(.horizontal) {
                            HStack(spacing: 4) {
                                ForEach(Array(layout.segments.enumerated()), id: \.element.id) { index, segment in
                                    PartitionSegmentButton(segment: segment, width: widths[index],
                                                           selected: selectedIdentifier == segment.id) { select(segment.id) }
                                        .id(segment.id)
                                        .focused($focusedSegment, equals: segment.id)
                                        .help(tooltip(segment))
                                        .accessibilityLabel(tooltip(segment))
                                        .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
                                        .onMoveCommand { moveSelection($0, from: segment.id) }
                                        .contextMenu { contextMenu(segment) }
                                }
                            }.padding(.vertical, 3)
                        }
                        .onChange(of: focusedSegment) { _, identifier in
                            if let identifier { proxy.scrollTo(identifier, anchor: .center) }
                        }
                        .onChange(of: selectedIdentifier) { _, identifier in
                            if layout.segments.contains(where: { $0.id == identifier }) { proxy.scrollTo(identifier, anchor: .center) }
                        }
                    }
                }.frame(height: 112)
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 14) { legend }
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 90), alignment: .leading)], alignment: .leading) { legend }
                }
                Text("Small partitions are widened for selection. Sizes are exact; gaps are not plotted. APFS volumes share their physical store.")
                    .font(.caption).foregroundStyle(.secondary)
                Divider().padding(.vertical, 2)
                HStack {
                    Text("Partitions").font(.headline)
                    Spacer()
                    Picker("Sort partitions", selection: $sort) {
                        ForEach(PartitionSort.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                    }.labelsHidden().fixedSize().controlSize(.small)
                }
                HStack {
                    Text("Name / identifier").frame(maxWidth: .infinity, alignment: .leading)
                    Text("Format").frame(width: 90, alignment: .leading)
                    Text("Capacity").frame(width: 85, alignment: .trailing)
                }
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 10)
                VStack(spacing: 2) {
                    ForEach(sortedSegments) { segment in
                        partitionRow(segment)
                    }
                }
            }
        }
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: selectedIdentifier)
    }

    private var legend: some View {
        ForEach(PartitionTone.allCases.filter { tone in layout.segments.contains { PartitionTone.tone(for: $0.partition) == tone } }, id: \.self) { tone in
            Label { Text(tone.rawValue) } icon: {
                Circle().fill(tone.color).frame(width: 7, height: 7)
            }.font(.caption).foregroundStyle(.secondary)
        }
    }

    private func partitionRow(_ segment: PartitionMapSegment) -> some View {
        Button { select(segment.id) } label: {
            HStack(spacing: 12) {
                RoundedRectangle(cornerRadius: 2).fill(PartitionTone.tone(for: segment.partition).color)
                    .frame(width: 8, height: 24).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    Text(segment.title).fontWeight(.medium).lineLimit(1)
                    Text(segment.partition.device.mountPoint ?? segment.id)
                        .font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
                    if let used = segment.partition.device.usedBytes, let available = segment.partition.device.availableBytes {
                        Text("\(Capacity.string(used)) used · \(Capacity.string(available)) available")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
                Text(segment.partition.device.filesystem?.name ?? PartitionPresentation(partition: segment.partition).typeName ?? "Not reported")
                    .font(.caption).foregroundStyle(.secondary).frame(width: 90, alignment: .leading)
                VStack(alignment: .trailing, spacing: 4) {
                    Text(Capacity.string(segment.partition.device.sizeBytes)).monospacedDigit()
                    if selectedIdentifier == segment.id {
                        Label("Selected", systemImage: "checkmark").font(.caption2).foregroundStyle(Color.accentColor)
                    }
                }.frame(width: 85, alignment: .trailing)
            }
            .padding(10).contentShape(Rectangle())
        }
        .buttonStyle(PartitionRowStyle(selected: selectedIdentifier == segment.id))
        .help(tooltip(segment))
        .accessibilityLabel(tooltip(segment))
        .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
        .contextMenu { contextMenu(segment) }
    }

    private func moveSelection(_ direction: MoveCommandDirection, from identifier: String) {
        guard direction == .left || direction == .right else { return }
        let current = layout.segments.firstIndex { $0.id == (focusedSegment ?? identifier) } ?? 0
        let next = direction == .left ? max(0, current - 1) : min(layout.segments.count - 1, current + 1)
        focusedSegment = layout.segments[next].id
        select(layout.segments[next].id)
    }

    private func tooltip(_ segment: PartitionMapSegment) -> String {
        let device = segment.partition.device
        return "\(segment.title) · \(PartitionPresentation(partition: segment.partition).category.rawValue)\n\(Capacity.string(device.sizeBytes))\(device.sizeBytes.map { " (\($0.formatted()) bytes)" } ?? "")\n\(device.filesystem?.name ?? "Filesystem not reported") · \(segment.id)\(device.mountPoint.map { "\n\($0)" } ?? "")"
    }

    @ViewBuilder private func contextMenu(_ segment: PartitionMapSegment) -> some View {
        if let node = store.nodes.flatMap(\.flattened).first(where: { $0.disk.device.identifier == disk.device.identifier && $0.device.identifier == segment.id }) {
            DeviceContextMenu(node: node, store: store)
        }
    }
}

private enum PartitionSort: String, CaseIterable {
    case position = "Physical order", name = "Name", capacity = "Largest first"
}

private struct PartitionSegmentButton: View {
    let segment: PartitionMapSegment
    let width: Double
    let selected: Bool
    let action: () -> Void
    @State private var hovered = false
    private var tone: Color { PartitionTone.tone(for: segment.partition).color }

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 6) {
                if width >= 110 {
                    HStack(spacing: 4) {
                        Text(segment.title).fontWeight(.semibold).lineLimit(1)
                        if selected { Image(systemName: "checkmark.circle.fill").foregroundStyle(Color.accentColor) }
                    }
                    Text(segment.partition.device.filesystem?.name ?? PartitionPresentation(partition: segment.partition).typeName ?? "Not reported")
                        .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    Spacer(minLength: 0)
                    Text(Capacity.string(segment.partition.device.sizeBytes)).font(.callout).monospacedDigit().lineLimit(1)
                } else {
                    Text(segment.partition.number.map(String.init) ?? "•").fontWeight(.semibold)
                    if selected { Image(systemName: "checkmark").font(.caption).foregroundStyle(Color.accentColor) }
                }
            }
            .padding(10)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(tone.opacity(hovered ? 0.16 : 0.09))
            .overlay(alignment: .top) { Rectangle().fill(tone).frame(height: 4) }
            .clipShape(RoundedRectangle(cornerRadius: 6))
            .overlay {
                RoundedRectangle(cornerRadius: 6).strokeBorder(selected ? Color.accentColor : Color.secondary.opacity(0.25), lineWidth: selected ? 2 : 1)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain).frame(width: CGFloat(width), height: 96)
        .onHover { hovered = $0 }
    }
}

private struct PartitionRowStyle: ButtonStyle {
    let selected: Bool
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(selected ? Color.accentColor.opacity(0.12) : configuration.isPressed ? Color.secondary.opacity(0.08) : .clear,
                        in: RoundedRectangle(cornerRadius: 6))
            .overlay { RoundedRectangle(cornerRadius: 6).strokeBorder(selected ? Color.accentColor.opacity(0.6) : .clear) }
    }
}

extension PartitionTone {
    var color: Color {
        switch self {
        case .ntfs, .other: .blue
        case .apfs: .purple
        case .fat, .recovery: .green
        case .exfat: .teal
        case .ext, .system: .orange
        case .metadata, .unknown: .gray
        }
    }
}
