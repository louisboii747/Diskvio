import SwiftUI

struct PartitionMapView: View {
    let disk: PhysicalDisk
    let selectedIdentifier: String
    @ObservedObject var store: DiskStore
    let select: (String) -> Void
    private var layout: PartitionMapLayout { PartitionMapLayout(disk: disk) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Physical Partitions").font(.headline)
                Spacer()
                Text(disk.schemeName).font(.caption).foregroundStyle(.secondary)
            }
            if layout.segments.isEmpty {
                Text("No physical partitions reported.").foregroundStyle(.secondary)
            } else {
                GeometryReader { geometry in
                    let widths = layout.visualWidths(availableWidth: Double(geometry.size.width))
                    ScrollView(.horizontal) {
                        HStack(spacing: 4) {
                            ForEach(Array(layout.segments.enumerated()), id: \.element.id) { index, segment in
                                Button { select(segment.id) } label: {
                                    VStack(alignment: .leading, spacing: 5) {
                                        if widths[index] >= 100 {
                                            Text(segment.title).font(.caption).fontWeight(.medium).lineLimit(1)
                                            Text(Capacity.string(segment.partition.device.sizeBytes)).font(.caption2).lineLimit(1)
                                        } else {
                                            Text(segment.partition.number.map { "\($0)" } ?? "\(index + 1)").font(.caption).fontWeight(.semibold)
                                        }
                                    }
                                    .padding(8)
                                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                                    .background(category(segment).color.opacity(0.2))
                                    .overlay(alignment: .top) { Rectangle().fill(category(segment).color).frame(height: 4) }
                                    .clipShape(RoundedRectangle(cornerRadius: 5))
                                    .overlay { RoundedRectangle(cornerRadius: 5).strokeBorder(selectedIdentifier == segment.id ? Color.accentColor : Color.secondary.opacity(0.25), lineWidth: selectedIdentifier == segment.id ? 2 : 1) }
                                    .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                                .frame(width: CGFloat(widths[index]), height: 64)
                                .help(tooltip(segment))
                                .accessibilityLabel(tooltip(segment))
                                .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
                                .contextMenu { contextMenu(segment) }
                            }
                        }.padding(.vertical, 2)
                    }
                }.frame(height: 84)
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 14) { legend }
                    VStack(alignment: .leading, spacing: 6) { legend }
                }
                Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 0) {
                    GridRow { Text("Partition"); Text("Format"); Text("Capacity") }
                        .font(.caption).foregroundStyle(.secondary).padding(.bottom, 6)
                    ForEach(layout.segments) { segment in
                        GridRow {
                            Button { select(segment.id) } label: {
                                HStack(spacing: 8) {
                                    Circle().fill(category(segment).color).frame(width: 8, height: 8).accessibilityHidden(true)
                                    VStack(alignment: .leading, spacing: 3) {
                                        Text(segment.title).fontWeight(.medium).lineLimit(2).multilineTextAlignment(.leading)
                                        Text(segment.partition.device.mountPoint ?? segment.id).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                                    }.frame(maxWidth: .infinity, alignment: .leading)
                                }
                                .padding(8)
                                .background(selectedIdentifier == segment.id ? Color.accentColor.opacity(0.12) : .clear, in: RoundedRectangle(cornerRadius: 5))
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .help(tooltip(segment))
                            .accessibilityLabel(tooltip(segment))
                            .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
                            .contextMenu { contextMenu(segment) }
                            Text(segment.partition.device.filesystem?.name ?? PartitionPresentation(partition: segment.partition).typeName ?? "Unknown")
                                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                            Text(Capacity.string(segment.partition.device.sizeBytes)).font(.callout).monospacedDigit().gridColumnAlignment(.trailing)
                        }
                    }
                }
                Text("Widths compare reported partition capacities, with a minimum size for small partitions. Gaps and unallocated regions are not plotted. APFS volumes share their physical store.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
    }

    private var legend: some View {
        ForEach(PartitionCategory.allCases.filter { type in layout.segments.contains { category($0) == type } }, id: \.self) { type in
            HStack(spacing: 5) {
                Circle().fill(type.color).frame(width: 7, height: 7).accessibilityHidden(true)
                Text(type.rawValue).font(.caption).foregroundStyle(.secondary)
            }
        }
    }
    private func category(_ segment: PartitionMapSegment) -> PartitionCategory { PartitionPresentation(partition: segment.partition).category }
    private func tooltip(_ segment: PartitionMapSegment) -> String {
        "\(segment.title) · \(category(segment).rawValue)\n\(Capacity.string(segment.partition.device.sizeBytes)) · \(segment.partition.device.filesystem?.name ?? "Filesystem not reported")\n\(segment.id)\(segment.partition.device.mountPoint.map { " · \($0)" } ?? "")"
    }
    @ViewBuilder private func contextMenu(_ segment: PartitionMapSegment) -> some View {
        if let node = store.nodes.flatMap(\.flattened).first(where: { $0.disk.device.identifier == disk.device.identifier && $0.device.identifier == segment.id }) {
            DeviceContextMenu(node: node, store: store)
        }
    }
}

private extension PartitionCategory {
    var color: Color {
        switch self {
        case .system: .orange
        case .recovery: .green
        case .metadata: .gray
        case .data: .blue
        case .unknown: Color(nsColor: .secondaryLabelColor)
        }
    }
}
