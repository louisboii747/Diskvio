import SwiftUI

struct PartitionMapView: View {
    let disk: PhysicalDisk
    let selectedIdentifier: String
    let select: (String) -> Void
    private let colors: [Color] = [.blue, .teal, .orange, .purple, .pink, .indigo]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Partition Map").font(.headline)
            Text(disk.schemeName).font(.callout).foregroundStyle(.secondary)
            if let total = disk.device.sizeBytes, total > 0, !disk.partitions.isEmpty {
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle().fill(.quaternary)
                        ForEach(Array(disk.partitions.enumerated()), id: \.element.device.identifier) { index, partition in
                            let start = offset(for: index)
                            let length = min(partition.device.sizeBytes ?? 0, total > start ? total - start : 0)
                            Button { select(partition.device.identifier) } label: {
                                Rectangle()
                                    .fill(colors[index % colors.count].opacity(selectedIdentifier == partition.device.identifier ? 1 : 0.75))
                            }
                            .buttonStyle(.plain)
                            .frame(width: geometry.size.width * Double(length) / Double(total))
                            .offset(x: geometry.size.width * Double(min(start, total)) / Double(total))
                            .help("\(partition.device.identifier): \(Capacity.string(partition.device.sizeBytes))")
                            .accessibilityLabel("\(partition.device.name), \(Capacity.string(partition.device.sizeBytes))")
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 5))
                }
                .frame(height: 38)
                ForEach(Array(disk.partitions.enumerated()), id: \.element.device.identifier) { index, partition in
                    Button { select(partition.device.identifier) } label: {
                        HStack(spacing: 8) {
                            Circle().fill(colors[index % colors.count]).frame(width: 8, height: 8)
                                .accessibilityHidden(true)
                            Text(partition.device.name)
                            if partition.device.name != partition.device.identifier {
                                Text(partition.device.identifier).foregroundStyle(.secondary)
                            }
                            Spacer(minLength: 8)
                            Text(Capacity.string(partition.device.sizeBytes)).monospacedDigit()
                        }
                        .font(.callout)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
                Text("Segment widths show physical partition capacity. Gray space includes gaps and partition-map metadata.")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                Text(disk.partitions.isEmpty ? "No physical partitions reported." : "Capacity is unavailable for this disk.")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func offset(for index: Int) -> UInt64 {
        if let offset = disk.partitions[index].offsetBytes { return offset }
        return disk.partitions.prefix(index).reduce(UInt64(0)) { result, partition in
            let sum = result.addingReportingOverflow(partition.device.sizeBytes ?? 0)
            return sum.overflow ? UInt64.max : sum.partialValue
        }
    }
}
