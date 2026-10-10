import Foundation

nonisolated struct PartitionMapLayout: Sendable {
    let totalBytes: UInt64?
    let segments: [PartitionMapSegment]

    init(disk: PhysicalDisk) {
        let total = disk.device.sizeBytes.flatMap { $0 > 0 ? $0 : nil }
        totalBytes = total
        var end: UInt64 = 0
        segments = disk.partitions.enumerated().map { index, partition in
            let start = partition.offsetBytes ?? end
            if let length = partition.device.sizeBytes {
                let next = start.addingReportingOverflow(length)
                end = next.overflow ? .max : next.partialValue
            } else {
                end = start
            }
            return PartitionMapSegment(partition: partition, index: index, startBytes: start, totalBytes: total)
        }
    }
    /// A schematic comparison of reported sizes, with no invented gap regions.
    func visualWidths(availableWidth: Double, minimumWidth: Double = 48, spacing: Double = 4) -> [Double] {
        guard !segments.isEmpty else { return [] }
        let count = Double(segments.count)
        let usable = max(availableWidth - spacing * (count - 1), minimumWidth * count)
        let remaining = max(0, usable - minimumWidth * count)
        let weights = segments.map { Double($0.partition.device.sizeBytes ?? 0) }
        let sum = weights.reduce(0, +)
        return weights.map { minimumWidth + remaining * (sum > 0 ? $0 / sum : 1 / count) }
    }
}

nonisolated struct PartitionMapSegment: Identifiable, Sendable {
    let partition: DiskPartition
    let index: Int
    let startBytes: UInt64
    let totalBytes: UInt64?

    var id: String { partition.device.identifier }
    var title: String {
        PartitionPresentation(partition: partition).name
    }
    var startFraction: Double {
        guard let totalBytes else { return 0 }
        return Double(min(startBytes, totalBytes)) / Double(totalBytes)
    }
    var lengthFraction: Double {
        guard let totalBytes, let length = partition.device.sizeBytes else { return 0 }
        return Double(min(length, totalBytes - min(startBytes, totalBytes))) / Double(totalBytes)
    }
}
