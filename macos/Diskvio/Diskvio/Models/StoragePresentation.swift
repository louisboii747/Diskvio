import Foundation

nonisolated struct PartitionPlanningContext: Hashable, Sendable {
    let selection: String?
    let diskIdentifier: String?
    let identityToken: String?
    let revision: UInt64
}

nonisolated struct PartitionPlanningPresentation: Sendable {
    let result: PartitionPlanningResult?
    let error: String?

    var title: String {
        if error != nil { return "Backend discovery failed" }
        guard let result else { return "Checking partition planning support…" }
        if result.validation.issues.contains(where: { $0.code == "unsupported_platform" }) {
            return "Partition planning is unavailable on macOS"
        }
        return result.validation.valid ? "Preview only" : "Partition planning unavailable"
    }

    var explanation: String {
        if let error { return error }
        guard let result else { return "Reading the backend capability for this physical disk." }
        if result.validation.issues.contains(where: { $0.code == "unsupported_platform" }) {
            return "The backend does not provide macOS partition allocation or create/delete previews. APFS free capacity is shared within its container and is not physical unallocated space."
        }
        return result.validation.issues.map(\.message).joined(separator: "\n")
    }

    var technicalDetails: String {
        result?.validation.issues.map { "\($0.code): \($0.message)" }.joined(separator: "\n") ?? ""
    }
}

nonisolated struct StoragePresentation: Sendable {
    let node: DeviceNode

    var protectionExplanation: String {
        if node.disk.internal == true { return "Internal disk. Available for inspection only." }
        if node.disk.internal == nil { return "Disk location is unknown. Management requires verified external media." }
        if node.device.safety?.system == true || node.device.safety?.boot == true || node.device.safety?.recovery == true {
            return "Protected system, boot or recovery target. Available for inspection only."
        }
        if node.device.safety?.readOnly == true || node.disk.device.safety?.readOnly == true { return "Read-only media. Writable operations are unavailable." }
        if node.hasManagementActions { return "Only backend-advertised volume operations are available. Each operation revalidates device identity and safety." }
        return "No management capability is reported for this selection. Select a supported external volume to inspect available actions."
    }

    var exactCapacity: String {
        node.device.sizeBytes.map { "\($0.formatted()) bytes" } ?? "Not available"
    }

    var partitionBounds: String {
        guard let partition = node.partition, let start = partition.offsetBytes,
              let size = partition.device.sizeBytes else { return "Not available" }
        let end = start.addingReportingOverflow(size)
        guard !end.overflow else { return "Invalid reported geometry" }
        return "[\(start.formatted()), \(end.partialValue.formatted())) bytes"
    }
}
