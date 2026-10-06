import Foundation

struct Disk: Decodable, Identifiable, Sendable {
    let number: UInt32
    let name: String
    let sizeBytes: UInt64
    let busType: String
    let partitionStyle: String

    var id: UInt32 { number }

    enum CodingKeys: String, CodingKey {
        case number, name
        case sizeBytes = "size_bytes"
        case busType = "bus_type"
        case partitionStyle = "partition_style"
    }
}
