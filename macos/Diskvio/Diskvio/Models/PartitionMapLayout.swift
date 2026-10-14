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
    func visualWidths(availableWidth: Double, minimumWidth: Double = 48, spacing: Double = 4) -> [Double] {
        guard !segments.isEmpty else { return [] }
        let count = Double(segments.count)
        let usable = max(availableWidth - spacing * (count - 1), minimumWidth * count)
        var remaining = usable
        let weights = segments.map { Double($0.partition.device.sizeBytes ?? 0) }
        var widths = Array(repeating: 0.0, count: segments.count)
        var pending = Array(segments.indices)
        while !pending.isEmpty {
            let total = pending.reduce(0) { $0 + weights[$1] }
            let small = pending.filter { total <= 0 || remaining * weights[$0] / total < minimumWidth }
            if small.isEmpty {
                for index in pending { widths[index] = remaining * weights[index] / total }
                break
            }
            for index in small {
                widths[index] = minimumWidth
                remaining -= minimumWidth
            }
            pending.removeAll { small.contains($0) }
        }
        return widths
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
