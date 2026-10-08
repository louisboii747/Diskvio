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
}

nonisolated struct PartitionMapSegment: Identifiable, Sendable {
    let partition: DiskPartition
    let index: Int
    let startBytes: UInt64
    let totalBytes: UInt64?

    var id: String { partition.device.identifier }
    var title: String {
        if partition.device.name != id { return partition.device.name }
        return partition.device.filesystem?.name ?? partition.contentType?.replacingOccurrences(of: "_", with: " ") ?? id
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
