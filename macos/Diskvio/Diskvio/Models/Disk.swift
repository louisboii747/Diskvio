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
            let diskID = "disk:\(disk.device.stableID ?? disk.device.partitionUUID ?? disk.device.identifier):\(disk.device.sizeBytes ?? 0)"
            var root = DeviceNode(id: diskID, kind: .disk, device: disk.device, disk: disk)
            root.children = disk.partitions.map { partition in
                let partitionID = "\(diskID)/partition:\(partition.device.partitionUUID ?? partition.device.identifier)"
                var node = DeviceNode(id: partitionID, kind: .partition, device: partition.device, disk: disk)
                node.contentType = partition.contentType
                node.partition = partition
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
    let volumeLabel: String?
    let stableID: String?
    let mountPoints: [String]?
    let safety: DeviceSafety?
    let healthStatus: String?
    let operationalStatus: [String]?

    var confirmedMountPoints: [String] { mountPoints ?? mountPoint.map { [$0] } ?? [] }

    enum CodingKeys: String, CodingKey {
        case identifier, name, filesystem
        case mediaName = "media_name", sizeBytes = "size_bytes", mountPoint = "mount_point"
        case volumeUUID = "volume_uuid", partitionUUID = "partition_uuid"
        case usedBytes = "used_bytes", availableBytes = "available_bytes"
        case identityToken = "identity_token", actions
        case volumeLabel = "volume_label", stableID = "stable_id", mountPoints = "mount_points", safety
        case healthStatus = "health_status", operationalStatus = "operational_status"
    }
}

nonisolated struct DeviceSafety: Decodable, Sendable {
    let system: Bool?
    let boot: Bool?
    let recovery: Bool?
    let hidden: Bool?
    let readOnly: Bool?
    let offline: Bool?
    enum CodingKeys: String, CodingKey {
        case system, boot, recovery, hidden, offline
        case readOnly = "read_only"
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
    let role: String?
    let number: UInt32?

    enum CodingKeys: String, CodingKey {
        case device, role, number
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
    var partition: DiskPartition?

    var displayName: String {
        if let partition { return PartitionPresentation(partition: partition).name }
        if let label = device.volumeLabel, !label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return label }
        if kind == .container { return "APFS Container" }
        if device.name != device.identifier && !device.name.isEmpty { return device.name }
        return kind == .disk ? "Physical Disk \(disk.number)" : "Unlabelled Volume"
    }

    var physicalPartitionIdentifier: String? {
        if kind == .partition { return device.identifier }
        return disk.partitions.first { $0.apfsContainerID == container?.device.identifier && $0.apfsContainerID != nil }?.device.identifier
    }

    var badges: [String] {
        var result: [String] = []
        if disk.internal == false { result.append("External") }
        if disk.removable == true { result.append("Removable") }
        if disk.connectionType?.localizedCaseInsensitiveContains("USB") == true { result.append("USB") }
        if device.safety?.system == true || volume?.roles.contains(where: { $0.caseInsensitiveCompare("System") == .orderedSame }) == true { result.append("System") }
        if device.safety?.boot == true || volume?.roles.contains(where: { $0.caseInsensitiveCompare("Preboot") == .orderedSame }) == true { result.append("Boot") }
        if device.safety?.recovery == true || partition?.role == "recovery" || volume?.roles.contains(where: { $0.caseInsensitiveCompare("Recovery") == .orderedSame }) == true { result.append("Recovery") }
        if partition?.role == "efi_system" || contentType == "EFI" { result.append("EFI") }
        if device.safety?.readOnly == true { result.append("Read-only") }
        if volume?.encrypted == true { result.append("Encrypted") }
        if volume?.locked == true { result.append("Locked") }
        return result
    }

    var capacityLabel: String {
        guard kind == .volume else { return "Capacity" }
        return volume?.quotaBytes == nil ? "Shared capacity" : "Capacity limit"
    }
    var capacityDescription: String {
        let capacity = Capacity.string(device.sizeBytes)
        guard kind == .volume else { return capacity }
        return "\(volume?.quotaBytes == nil ? "Shared" : "Limited") \(capacity)"
    }

    var hasManagementActions: Bool {
        (device.actions ?? []).contains { operationTarget(for: $0) != nil } || operationTarget(for: .eject) != nil
    }

    func operationTarget(for action: DiskAction) -> DeviceNode? {
        guard action != .setDriveLetter else { return nil }
        if device.identityToken != nil, device.actions?.contains(action) == true { return self }
        guard action == .eject, kind != .disk, disk.device.identityToken != nil,
              disk.device.actions?.contains(.eject) == true else { return nil }
        return DeviceNode(id: id, kind: .disk, device: disk.device, disk: disk)
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
    case renameVolume = "rename_volume"
    case setDriveLetter = "set_drive_letter"
    var title: String {
        switch self { case .mount: "Mount"; case .unmount: "Unmount"; case .eject: "Eject"; case .renameVolume: "Rename Volume"; case .setDriveLetter: "Change Drive Letter" }
    }
    var symbol: String {
        switch self { case .mount: "externaldrive.badge.plus"; case .unmount: "externaldrive.badge.minus"; case .eject: "eject"; case .renameVolume: "pencil"; case .setDriveLetter: "character.cursor.ibeam" }
    }
}

nonisolated struct PendingDiskOperation: Identifiable, Sendable {
    let action: DiskAction
    let node: DeviceNode

    var id: String { "\(action.rawValue):\(node.device.identifier)" }

    var title: String {
        switch action {
        case .mount: "Mount \(node.displayName)?"
        case .unmount: "Unmount \(node.displayName)?"
        case .eject: "Eject \(node.displayName)?"
        case .renameVolume: "Rename \(node.displayName)?"
        case .setDriveLetter: "Change drive letter?"
        }
    }

    var message: String {
        switch action {
        case .mount:
            "Diskvio will ask macOS to mount this volume."
        case .unmount:
            "Open files on this volume may become unavailable. Save your work and close files stored on it before continuing."
        case .eject:
            "All volumes on this disk will become unavailable. Save your work and close files stored on the disk before continuing."
        case .renameVolume: "The volume label and its Finder mount path may change."
        case .setDriveLetter: "Drive letters apply only to Windows."
        }
    }

    var confirmationTitle: String {
        switch action {
        case .mount: "Mount"
        case .unmount: "Unmount"
        case .eject: "Eject"
        case .renameVolume: "Rename"
        case .setDriveLetter: "Change"
        }
    }
}

nonisolated struct OperationRequest: Encodable, Sendable {
    let action: DiskAction
    let identifier: String
    let expectedIdentity: String
    var volumeLabel: String?
    var expectedMountPoints: [String]?
    enum CodingKeys: String, CodingKey {
        case action, identifier
        case expectedIdentity = "expected_identity"
        case volumeLabel = "volume_label", expectedMountPoints = "expected_mount_points"
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

nonisolated struct BackendOperationError: Decodable, Sendable {
    let code: String
    let message: String
    let platformCode: Int64?
    enum CodingKeys: String, CodingKey { case code, message; case platformCode = "platform_code" }
}

nonisolated struct OperationCapabilities: Decodable, Sendable {
    let identifier: String
    let deviceKind: String
    let identityToken: String?
    let actions: [DiskAction]
    var unsupportedReason: String?
    var labelMaxLength: Int?
    var limitations: [String]?
    enum CodingKeys: String, CodingKey {
        case identifier, actions
        case deviceKind = "device_kind", identityToken = "identity_token"
        case unsupportedReason = "unsupported_reason", labelMaxLength = "label_max_length", limitations
    }
}

nonisolated struct OperationValidation: Decodable, Sendable {
    let valid: Bool
    let action: DiskAction
    let identifier: String
    let expectedIdentity: String
    enum CodingKeys: String, CodingKey { case valid, action, identifier; case expectedIdentity = "expected_identity" }
}

nonisolated struct OperationQueryRequest: Encodable, Sendable {
    let mode: String
    let action: DiskAction?
    let identifier: String
    let expectedIdentity: String?
    var volumeLabel: String?
    var expectedMountPoints: [String]?
    enum CodingKeys: String, CodingKey {
        case mode, action, identifier
        case expectedIdentity = "expected_identity", volumeLabel = "volume_label", expectedMountPoints = "expected_mount_points"
    }
}
