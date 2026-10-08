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

nonisolated struct DiskInventory: Decodable, Sendable {
    var disks: [PhysicalDisk] = []
    var apfsContainers: [APFSContainer] = []
    var warnings: [String] = []

    enum CodingKeys: String, CodingKey {
        case disks, warnings
        case apfsContainers = "apfs_containers"
    }

    var nodes: [DeviceNode] {
        disks.map { disk in
            let diskID = "disk:\(disk.device.identifier):\(disk.device.name):\(disk.device.sizeBytes ?? 0)"
            var root = DeviceNode(id: diskID, kind: .disk, device: disk.device, disk: disk)
            root.children = disk.partitions.map { partition in
                let partitionID = "\(diskID)/partition:\(partition.device.partitionUUID ?? partition.device.identifier)"
                var node = DeviceNode(id: partitionID, kind: .partition, device: partition.device, disk: disk)
                node.contentType = partition.contentType
                if let container = apfsContainers.first(where: { $0.device.identifier == partition.apfsContainerID }) {
                    let containerID = "\(partitionID)/container:\(container.uuid ?? container.device.identifier)"
                    var child = DeviceNode(id: containerID, kind: .container, device: container.device, disk: disk)
                    child.container = container
                    child.children = container.volumes.map { volume in
                        var node = DeviceNode(
                            id: "\(containerID)/volume:\(volume.device.volumeUUID ?? volume.device.identifier)",
                            kind: .volume, device: volume.device, disk: disk
                        )
                        node.container = container
                        node.volume = volume
                        return node
                    }
                    if child.children?.isEmpty == true { child.children = nil }
                    node.children = [child]
                }
                return node
            }
            // An unpartitioned physical APFS store may back a container directly.
            for container in apfsContainers where container.physicalStoreIDs.contains(disk.device.identifier) {
                var node = DeviceNode(id: "\(diskID)/container:\(container.uuid ?? container.device.identifier)", kind: .container, device: container.device, disk: disk)
                node.container = container
                node.children = container.volumes.map { volume in
                    var child = DeviceNode(id: "\(node.id)/volume:\(volume.device.volumeUUID ?? volume.device.identifier)", kind: .volume, device: volume.device, disk: disk)
                    child.container = container
                    child.volume = volume
                    return child
                }
                if node.children?.isEmpty == true { node.children = nil }
                root.children?.append(node)
            }
            if root.children?.isEmpty == true { root.children = nil }
            return root
        }
    }
}

nonisolated struct DeviceInfo: Decodable, Sendable {
    let identifier: String
    let name: String
    let mediaName: String?
    let sizeBytes: UInt64?
    let filesystem: FilesystemInfo?
    let mountPoint: String?
    let volumeUUID: String?
    let partitionUUID: String?
    let usedBytes: UInt64?
    let availableBytes: UInt64?
    let identityToken: String?
    let actions: [DiskAction]?

    enum CodingKeys: String, CodingKey {
        case identifier, name, filesystem
        case mediaName = "media_name", sizeBytes = "size_bytes", mountPoint = "mount_point"
        case volumeUUID = "volume_uuid", partitionUUID = "partition_uuid"
        case usedBytes = "used_bytes", availableBytes = "available_bytes"
        case identityToken = "identity_token", actions
    }
}

nonisolated struct FilesystemInfo: Decodable, Sendable {
    let name: String
    let kind: String
}

nonisolated struct PhysicalDisk: Decodable, Sendable {
    let device: DeviceInfo
    let number: UInt32
    let partitionScheme: String
    let connectionType: String?
    let `internal`: Bool?
    let removable: Bool?
    let ejectable: Bool?
    var partitions: [DiskPartition]

    enum CodingKeys: String, CodingKey {
        case device, number, `internal`, removable, ejectable, partitions
        case partitionScheme = "partition_scheme", connectionType = "connection_type"
    }

    var schemeName: String {
        switch partitionScheme {
        case "gpt": "GUID Partition Map (GPT)"
        case "mbr": "Master Boot Record (MBR)"
        case "apple_partition_map": "Apple Partition Map"
        case "none": "No partition map"
        default: "Unknown"
        }
    }
}

nonisolated struct DiskPartition: Decodable, Sendable {
    let device: DeviceInfo
    let contentType: String?
    let offsetBytes: UInt64?
    let apfsContainerID: String?

    enum CodingKeys: String, CodingKey {
        case device
        case contentType = "content_type", offsetBytes = "offset_bytes", apfsContainerID = "apfs_container_id"
    }
}

nonisolated struct APFSContainer: Decodable, Sendable {
    let device: DeviceInfo
    let uuid: String?
    let physicalStoreIDs: [String]
    var volumes: [APFSVolume]

    enum CodingKeys: String, CodingKey {
        case device, uuid, volumes
        case physicalStoreIDs = "physical_store_ids"
    }
}

nonisolated struct APFSVolume: Decodable, Sendable {
    let device: DeviceInfo
    let mountedSnapshots: [APFSSnapshot]?
    let roles: [String]
    let encrypted: Bool?
    let locked: Bool?
    let quotaBytes: UInt64?
    let reserveBytes: UInt64?

    enum CodingKeys: String, CodingKey {
        case device, roles, encrypted, locked
        case quotaBytes = "quota_bytes", reserveBytes = "reserve_bytes"
        case mountedSnapshots = "mounted_snapshots"
    }
}

nonisolated enum DeviceKind: String, Sendable {
    case disk = "Physical Disk", partition = "Partition", container = "APFS Container", volume = "APFS Volume"

    var symbol: String {
        switch self {
        case .disk: "externaldrive"
        case .partition: "rectangle.split.3x1"
        case .container: "square.stack.3d.up"
        case .volume: "internaldrive"
        }
    }
}

nonisolated struct DeviceNode: Identifiable, Sendable {
    let id: String
    let kind: DeviceKind
    let device: DeviceInfo
    let disk: PhysicalDisk
    var children: [DeviceNode]?
    var contentType: String?
    var container: APFSContainer?
    var volume: APFSVolume?

    var capacityLabel: String {
        guard kind == .volume else { return "Capacity" }
        return volume?.quotaBytes == nil ? "Shared capacity" : "Capacity limit"
    }
    var capacityDescription: String {
        let capacity = Capacity.string(device.sizeBytes)
        guard kind == .volume else { return capacity }
        return "\(volume?.quotaBytes == nil ? "Shared" : "Limited") \(capacity)"
    }

    var flattened: [DeviceNode] { [self] + (children ?? []).flatMap(\.flattened) }
    var uuid: String? { containerUUID ?? device.volumeUUID ?? device.partitionUUID }
    private var containerUUID: String? { kind == .container ? container?.uuid : nil }
}

nonisolated enum Capacity {
    static func string(_ bytes: UInt64?) -> String {
        guard let bytes else { return "Not available" }
        return ByteCountFormatter.string(fromByteCount: Int64(clamping: bytes), countStyle: .decimal)
    }
}


nonisolated enum DiskAction: String, Codable, Sendable {
    case mount, unmount, eject
    var title: String {
        switch self { case .mount: "Mount"; case .unmount: "Unmount"; case .eject: "Eject" }
    }
    var symbol: String {
        switch self { case .mount: "externaldrive.badge.plus"; case .unmount: "externaldrive.badge.minus"; case .eject: "eject" }
    }
}

nonisolated struct OperationRequest: Encodable, Sendable {
    let action: DiskAction
    let identifier: String
    let expectedIdentity: String
    enum CodingKeys: String, CodingKey {
        case action, identifier
        case expectedIdentity = "expected_identity"
    }
}

nonisolated struct OperationOutcome: Decodable, Sendable {
    let action: DiskAction
    let identifier: String
    let message: String
}


extension PhysicalDisk {
    mutating func removePartition(identifier: String) { partitions.removeAll { $0.device.identifier == identifier } }
}
extension APFSContainer {
    mutating func removeVolume(identifier: String) { volumes.removeAll { $0.device.identifier == identifier } }
}


nonisolated struct APFSSnapshot: Decodable, Sendable {
    let identifier: String
    let name: String?
    let uuid: String?
    let mountPoint: String?
    enum CodingKeys: String, CodingKey { case identifier, name, uuid; case mountPoint = "mount_point" }
}
