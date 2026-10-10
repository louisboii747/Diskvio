import Foundation

nonisolated enum PartitionCategory: String, CaseIterable, Sendable {
    case system = "System / EFI"
    case recovery = "Recovery"
    case metadata = "Reserved / Metadata"
    case data = "Data"
    case unknown = "Unknown"
}

nonisolated struct PartitionPresentation: Sendable {
    let partition: DiskPartition

    var typeName: String? {
        switch partition.contentType {
        case "Apple_APFS": "APFS Physical Store"
        case "Apple_HFS": "Apple HFS Partition"
        case "Apple_partition_map": "Apple Partition Map"
        case "Apple_Free": "Apple Free Partition"
        default:
            switch partition.role {
            case "efi_system": "EFI System Partition"
            case "recovery": "Recovery Partition"
            case "microsoft_reserved": "Microsoft Reserved"
            case "basic_data": "Basic Data"
            default:
                switch partition.contentType {
                case "EFI": "EFI System Partition"
                case "Apple_Boot", "Apple_Recovery": "Recovery Partition"
                case "Microsoft Basic Data": "Basic Data"
                default: nil
                }
            }
        }
    }

    var name: String {
        if let label = partition.device.volumeLabel, !label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return label }
        // Older backends used a real filesystem label as `name`.
        let device = partition.device
        if device.filesystem != nil, device.name != device.identifier, device.name != device.mediaName,
           !device.name.isEmpty, !device.name.hasPrefix("Partition ") { return device.name }
        if let typeName { return typeName }
        let number = partition.number ?? partition.device.identifier.split(separator: "s").last.flatMap { UInt32($0) }
        return number.map { "Partition \($0)" } ?? "Partition"
    }

    var category: PartitionCategory {
        if partition.device.safety?.system == true || partition.device.safety?.boot == true || partition.role == "efi_system" || partition.contentType == "EFI" { return .system }
        if partition.device.safety?.recovery == true || partition.role == "recovery" || ["Apple_Boot", "Apple_Recovery"].contains(partition.contentType) { return .recovery }
        if partition.role == "microsoft_reserved" || partition.contentType == "Apple_partition_map" { return .metadata }
        if partition.role == "basic_data" || partition.device.filesystem != nil || ["Apple_APFS", "Apple_HFS", "Microsoft Basic Data"].contains(partition.contentType) { return .data }
        return .unknown
    }
}
