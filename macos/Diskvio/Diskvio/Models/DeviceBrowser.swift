import Foundation

nonisolated enum DeviceGroup: String, CaseIterable, Identifiable, Sendable {
    case internalDrives = "Internal"
    case externalDrives = "External"
    case removable = "Removable"
    case other = "Other devices"

    var id: String { rawValue }
    static func group(for disk: PhysicalDisk) -> DeviceGroup {
        if disk.internal == true { return .internalDrives }
        if disk.removable == true { return .removable }
        if disk.internal == false { return .externalDrives }
        return .other
    }
}

nonisolated enum DeviceFilter: String, CaseIterable, Identifiable, Sendable {
    case all = "All devices"
    case internalDrives = "Internal drives"
    case externalDrives = "External drives"
    case removable = "Removable devices"
    var id: String { rawValue }

    func includes(_ disk: PhysicalDisk) -> Bool {
        switch self {
        case .all: true
        case .internalDrives: disk.internal == true
        case .externalDrives: disk.internal == false
        case .removable: disk.removable == true
        }
    }
}

extension DeviceNode {
    func matches(_ query: String) -> Bool {
        flattened.contains { $0.matchesSelf(query) }
    }

    func matchesSelf(_ query: String) -> Bool {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        return query.isEmpty || [displayName, device.identifier, device.mediaName, device.mountPoint,
             device.filesystem?.name, disk.manufacturer, disk.model, disk.connectionType]
                .compactMap { $0 }.contains { $0.localizedStandardContains(query) }
    }

    var deviceSymbol: String {
        guard kind == .disk else { return kind.symbol }
        if disk.internal == true { return "internaldrive" }
        if disk.connectionType?.localizedCaseInsensitiveContains("USB") == true { return "externaldrive.connected.to.line.below" }
        return "externaldrive"
    }

    var connectionDescription: String {
        let location = disk.internal.map { $0 ? "Internal" : "External" } ?? "Location unknown"
        return [location, disk.connectionType].compactMap { $0 }.joined(separator: " · ")
    }

    var reportedStatus: String {
        if device.safety?.offline == true { return "Offline" }
        if volume?.locked == true { return "Locked" }
        if let status = device.operationalStatus, !status.isEmpty { return status.joined(separator: ", ") }
        if let health = device.healthStatus, !health.isEmpty { return health }
        if device.mountPoint != nil { return "Mounted" }
        if device.safety?.readOnly == true { return "Read-only" }
        return kind == .disk ? "Status not reported" : "Not mounted directly"
    }
}

nonisolated enum PartitionTone: String, CaseIterable, Sendable {
    case ntfs = "NTFS", apfs = "APFS", fat = "FAT", exfat = "exFAT", ext = "ext"
    case system = "System", recovery = "Recovery", metadata = "Metadata", other = "Other", unknown = "Unknown"

    static func tone(for partition: DiskPartition) -> PartitionTone {
        let filesystem = partition.device.filesystem
        let value = "\(filesystem?.kind ?? "") \(filesystem?.name ?? "")".lowercased()
        if value.contains("ntfs") { return .ntfs }
        if value.contains("apfs") || ["Apple_APFS", "Apple_APFS_ISC", "Apple_APFS_Recovery"].contains(partition.contentType) { return .apfs }
        if value.contains("exfat") { return .exfat }
        if value.contains("fat") || value.contains("msdos") { return .fat }
        if ["ext2", "ext3", "ext4"].contains(where: value.contains) { return .ext }
        switch PartitionPresentation(partition: partition).category {
        case .system: return .system
        case .recovery: return .recovery
        case .metadata: return .metadata
        case .data: return .other
        case .unknown: return .unknown
        }
    }
}
