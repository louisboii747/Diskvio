import Foundation

nonisolated struct PhysicalDiskIdentity: Codable, Sendable, Equatable {
    let uniqueId: String
    let uniqueIdFormat: UInt16
    let serialNumber: String
}

nonisolated struct DiskLayoutSnapshot: Codable, Sendable, Equatable {
    let version: UInt32
    let token: String
}

nonisolated struct AllocatableRegionIdentity: Codable, Sendable {
    let startOffsetBytes: UInt64
    let endOffsetBytes: UInt64
}

nonisolated struct TestDiskDesignation: Encodable, Sendable {
    let identity: PhysicalDiskIdentity
    let purpose = "expendable_external_test_disk"
}

nonisolated struct CreatePartitionPlan: Encodable, Sendable {
    let identifier: String
    let expectedIdentity: PhysicalDiskIdentity
    let expectedLayout: DiskLayoutSnapshot
    let testDisk: TestDiskDesignation
    let region: AllocatableRegionIdentity
    let requestedSizeBytes: UInt64
    var requestedStartOffsetBytes: UInt64?
    let requiredAlignmentBytes: UInt64
    let partitionTable: String
    let proposedGptType: String
}

nonisolated struct PartitionTarget: Codable, Sendable {
    let partitionId: String
    let number: UInt32
    let offsetBytes: UInt64
    let sizeBytes: UInt64
    let gptType: String
}

nonisolated struct DeletePartitionPlan: Encodable, Sendable {
    let identifier: String
    let expectedIdentity: PhysicalDiskIdentity
    let expectedLayout: DiskLayoutSnapshot
    let testDisk: TestDiskDesignation
    let partition: PartitionTarget
}

nonisolated enum PartitionPlanningRequest: Encodable, Sendable {
    case discover(identifier: String)
    case create(CreatePartitionPlan)
    case delete(DeletePartitionPlan)

    private enum CodingKeys: String, CodingKey { case mode, identifier }

    func encode(to encoder: any Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .discover(let identifier):
            try container.encode("discover", forKey: .mode)
            try container.encode(identifier, forKey: .identifier)
        case .create(let plan):
            try container.encode("create", forKey: .mode)
            try plan.encode(to: encoder)
        case .delete(let plan):
            try container.encode("delete", forKey: .mode)
            try plan.encode(to: encoder)
        }
    }

    func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return try encoder.encode(self)
    }
}

nonisolated struct PlanningIssue: Decodable, Sendable {
    let code: String
    let message: String
    let platformCode: Int64?
}

nonisolated struct PlanningValidation: Decodable, Sendable {
    let valid: Bool
    let issues: [PlanningIssue]
}

nonisolated struct GptGeometry: Decodable, Sendable {
    let diskGuid: String
    let diskSizeBytes: UInt64
    let usableStartBytes: UInt64
    let usableEndBytes: UInt64
    let logicalSectorBytes: UInt32
    let physicalSectorBytes: UInt32
    let sectorAlignmentOffsetBytes: UInt32
    let maxPartitionCount: UInt32
}

nonisolated struct PartitionDiskSafety: Decodable, Sendable {
    let external: Bool?
    let system: Bool?
    let boot: Bool?
    let bootFromDisk: Bool?
    let readOnly: Bool?
    let offline: Bool?
    let clustered: Bool?
    let storagePool: Bool?
    let dynamic: Bool?
}

nonisolated struct PlanningDeviceSafety: Decodable, Sendable {
    let system: Bool?
    let boot: Bool?
    let recovery: Bool?
    let hidden: Bool?
    let readOnly: Bool?
    let offline: Bool?
    let pageFile: Bool?
}

nonisolated struct PartitionSafety: Decodable, Sendable {
    let flags: PlanningDeviceSafety
    let active: Bool?
    let shadowCopy: Bool?
    let encrypted: Bool?
    let hostsWindows: Bool?
    let volumesComplete: Bool?
}

nonisolated struct PartitionVolumeReference: Decodable, Sendable {
    let identifier: String
    let mountPoints: [String]
}

nonisolated struct PlanningPartition: Decodable, Sendable {
    let partitionId: String?
    let number: UInt32?
    let offsetBytes: UInt64?
    let sizeBytes: UInt64?
    let gptType: String?
    let attributes: UInt64?
    let servicePartition: Bool?
    let name: String?
    let safety: PartitionSafety
    let volumes: [PartitionVolumeReference]
}

nonisolated struct AllocatableRegion: Decodable, Sendable {
    let parentIdentity: PhysicalDiskIdentity
    let identity: AllocatableRegionIdentity
    let capacityBytes: UInt64
    let requiredAlignmentBytes: UInt64
    let firstUsableAlignedOffsetBytes: UInt64?
    let lastUsableAlignedBoundaryBytes: UInt64?
    let maximumAllocatableBytes: UInt64
    let alignedCapacityBytes: UInt64
    let partitionTable: String
    let validation: PlanningValidation
}

nonisolated struct CreatePartitionPreview: Decodable, Sendable {
    let parentIdentity: PhysicalDiskIdentity
    let region: AllocatableRegionIdentity
    let expectedLayout: DiskLayoutSnapshot
    let partitionTable: String
    let proposedGptType: String
    let requiredAlignmentBytes: UInt64
    let startOffsetBytes: UInt64
    let endOffsetBytes: UInt64
    let allocatedSizeBytes: UInt64
    let remainingRegionCapacityBytes: UInt64
    let remainingAllocatableCapacityBytes: UInt64
}

nonisolated struct DeletePartitionPreview: Decodable, Sendable {
    let parentDisk: String
    let parentIdentity: PhysicalDiskIdentity?
    let partition: PlanningPartition
    let eligible: Bool
    let validation: PlanningValidation
    let dataLossWarning: String
}

nonisolated struct DiskPlanningDiscovery: Decodable, Sendable {
    let identifier: String
    let name: String
    let physicalIdentity: PhysicalDiskIdentity?
    let partitionTable: String
    let geometry: GptGeometry?
    let layoutSnapshot: DiskLayoutSnapshot?
    let safety: PartitionDiskSafety
    let validation: PlanningValidation
    let regions: [AllocatableRegion]
    let partitions: [DeletePartitionPreview]
}

nonisolated enum PartitionPlanningContractError: LocalizedError {
    case invalidResponse
    case backend(PlanningIssue)

    var errorDescription: String? {
        switch self {
        case .invalidResponse: "The backend returned an invalid preview-only partition contract."
        case .backend(let issue): issue.message
        }
    }
}

nonisolated struct PartitionPlanningResult: Decodable, Sendable {
    let apiVersion: UInt32
    let previewOnly: Bool
    let executionSupported: Bool
    let validation: PlanningValidation
    let discovery: DiskPlanningDiscovery?
    let createPreview: CreatePartitionPreview?
    let deletePreview: DeletePartitionPreview?

    static func decode(_ data: Data) throws -> Self {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let envelope = try decoder.decode(PartitionPlanningEnvelope.self, from: data)
        if envelope.status == "error", let issue = envelope.error { throw PartitionPlanningContractError.backend(issue) }
        guard envelope.status == "ok", let result = envelope.planning,
              result.apiVersion == 1, result.previewOnly, !result.executionSupported,
              result.validation.valid == result.validation.issues.isEmpty else {
            throw PartitionPlanningContractError.invalidResponse
        }
        return result
    }
}

private nonisolated struct PartitionPlanningEnvelope: Decodable {
    let status: String
    let planning: PartitionPlanningResult?
    let error: PlanningIssue?
}
