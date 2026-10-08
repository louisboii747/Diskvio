import SwiftUI

struct PartitionMapView: View {
    let disk: PhysicalDisk
    let selectedIdentifier: String
    let select: (String) -> Void
    private let colors: [Color] = [.blue, .teal, .orange, .purple, .pink, .indigo]
    private var layout: PartitionMapLayout { PartitionMapLayout(disk: disk) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Partition Map").font(.headline)
            Text(disk.schemeName).font(.callout).foregroundStyle(.secondary)
            if layout.totalBytes != nil, !layout.segments.isEmpty {
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle().fill(.quaternary)
                        ForEach(layout.segments) { segment in
                            Button { select(segment.id) } label: {
                                Rectangle()
                                    .fill(color(for: segment).opacity(selectedIdentifier == segment.id ? 1 : 0.75))
                                    .overlay {
                                        if selectedIdentifier == segment.id {
                                            Rectangle().strokeBorder(.primary, lineWidth: 2)
                                        }
                                    }
                            }
                            .buttonStyle(.plain)
                            .frame(width: geometry.size.width * segment.lengthFraction)
                            .offset(x: geometry.size.width * segment.startFraction)
                            .help("\(segment.title) — \(segment.id), \(Capacity.string(segment.partition.device.sizeBytes))")
                            .accessibilityLabel("\(segment.title), \(segment.id)")
                            .accessibilityValue(Capacity.string(segment.partition.device.sizeBytes))
                            .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 5))
                }
                .frame(height: 38)
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), alignment: .leading)], alignment: .leading, spacing: 12) {
                    ForEach(layout.segments) { segment in
                        Button { select(segment.id) } label: {
                            HStack(alignment: .top, spacing: 8) {
                                Circle().fill(color(for: segment)).frame(width: 8, height: 8)
                                    .padding(.top, 5).accessibilityHidden(true)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(segment.title).lineLimit(2).multilineTextAlignment(.leading)
                                    HStack(spacing: 8) {
                                        if segment.title != segment.id {
                                            Text(segment.id).lineLimit(1)
                                        }
                                        Text(Capacity.string(segment.partition.device.sizeBytes)).monospacedDigit()
                                    }
                                    .font(.caption).foregroundStyle(.secondary)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .font(.callout)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .help("\(segment.title) — \(segment.id)")
                        .accessibilityLabel("\(segment.title), \(segment.id)")
                        .accessibilityValue(Capacity.string(segment.partition.device.sizeBytes))
                        .accessibilityAddTraits(selectedIdentifier == segment.id ? .isSelected : [])
                    }
                }
                Text("Segment widths show physical partition capacity. Gray space includes gaps and partition-map metadata.")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                Text(disk.partitions.isEmpty ? "No physical partitions reported." : "Capacity is unavailable for this disk.")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func color(for segment: PartitionMapSegment) -> Color { colors[segment.index % colors.count] }
}
