import Foundation
import Testing
@testable import DiskvioPresentation

private func planningFixture() throws -> Data {
    let root = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()
    return try Data(contentsOf: root.appendingPathComponent("crates/diskvio-core/tests/fixtures/partition_planning_contract.json"))
}

struct PartitionPlanningTests {
    @Test func sharedRustContractPreservesGeometrySafetyAndWarnings() throws {
        let result = try PartitionPlanningResult.decode(planningFixture())
        #expect(result.previewOnly)
        #expect(!result.executionSupported)
        #expect(result.discovery?.geometry?.diskSizeBytes == 104_857_600)
        #expect(result.discovery?.regions.first?.maximumAllocatableBytes == 98_566_144)
        #expect(result.createPreview?.allocatedSizeBytes == 4_194_304)
        #expect(result.discovery?.partitions.first?.partition.safety.flags.pageFile == false)
        #expect(result.discovery?.partitions.first?.partition.safety.encrypted == false)
        #expect(result.discovery?.partitions.first?.dataLossWarning.contains("permanent data loss") == true)
    }

    @Test func createRequestUsesPreviewModeAndPreservesExactUnsignedBytes() throws {
        let result = try PartitionPlanningResult.decode(planningFixture())
        let discovery = try #require(result.discovery)
        let identity = try #require(discovery.physicalIdentity)
        let snapshot = try #require(discovery.layoutSnapshot)
        let region = try #require(discovery.regions.first)
        let request = PartitionPlanningRequest.create(CreatePartitionPlan(
            identifier: discovery.identifier, expectedIdentity: identity, expectedLayout: snapshot,
            testDisk: TestDiskDesignation(identity: identity), region: region.identity,
            requestedSizeBytes: 9_007_199_254_740_993, requiredAlignmentBytes: 1_048_576,
            partitionTable: "gpt", proposedGptType: "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7"
        ))
        let json = try #require(JSONSerialization.jsonObject(with: request.encoded()) as? [String: Any])
        #expect(json["mode"] as? String == "create")
        #expect((json["requested_size_bytes"] as? NSNumber)?.uint64Value == 9_007_199_254_740_993)
        #expect((json["expected_layout"] as? [String: Any])?["token"] as? String == snapshot.token)
        #expect(json["action"] == nil)
        #expect(json["requested_start_offset_bytes"] == nil)
    }

    @Test func deleteAndDiscoveryUseSeparateTypedRequests() throws {
        let result = try PartitionPlanningResult.decode(planningFixture())
        let discovery = try #require(result.discovery)
        let identity = try #require(discovery.physicalIdentity)
        let snapshot = try #require(discovery.layoutSnapshot)
        let request = PartitionPlanningRequest.delete(DeletePartitionPlan(
            identifier: discovery.identifier, expectedIdentity: identity, expectedLayout: snapshot,
            testDisk: TestDiskDesignation(identity: identity),
            partition: PartitionTarget(partitionId: "partition", number: 1, offsetBytes: 1_048_576,
                                       sizeBytes: 4_194_304, gptType: "basic")
        ))
        let json = try #require(JSONSerialization.jsonObject(with: request.encoded()) as? [String: Any])
        #expect(json["mode"] as? String == "delete")
        #expect(json["region"] == nil)
        let query = try #require(JSONSerialization.jsonObject(with: PartitionPlanningRequest.discover(identifier: "disk2").encoded()) as? [String: Any])
        #expect(query.count == 2)
        #expect(query["mode"] as? String == "discover")
    }

    @Test(arguments: ["preview_only", "execution_supported", "api_version"])
    func rejectsExecutionOrUnknownVersion(field: String) throws {
        var json = try #require(JSONSerialization.jsonObject(with: planningFixture()) as? [String: Any])
        var planning = try #require(json["planning"] as? [String: Any])
        switch field {
        case "api_version": planning[field] = 2
        case "preview_only": planning[field] = false
        default: planning[field] = true
        }
        json["planning"] = planning
        let data = try JSONSerialization.data(withJSONObject: json)
        #expect(throws: PartitionPlanningContractError.self) { try PartitionPlanningResult.decode(data) }
    }

    @Test func unsupportedPlatformAndNativePermissionCodesStayVisible() throws {
        let data = Data(#"{"status":"ok","planning":{"api_version":1,"preview_only":true,"execution_supported":false,"validation":{"valid":false,"issues":[{"code":"unsupported_platform","message":"Unsupported on macOS","platform_code":5}]}}}"#.utf8)
        let result = try PartitionPlanningResult.decode(data)
        #expect(!result.validation.valid)
        #expect(result.validation.issues.first?.code == "unsupported_platform")
        #expect(result.validation.issues.first?.platformCode == 5)
        let presentation = PartitionPlanningPresentation(result: result, error: nil)
        #expect(presentation.title == "Partition planning is unavailable on macOS")
        #expect(presentation.explanation.contains("not physical unallocated space"))
        #expect(presentation.technicalDetails.contains("unsupported_platform"))
    }

    @Test func discoveryFailuresAndPendingQueriesAreDistinctFromUnsupported() {
        let pending = PartitionPlanningPresentation(result: nil, error: nil)
        #expect(pending.title.contains("Checking"))
        let failed = PartitionPlanningPresentation(result: nil, error: "Disconnected")
        #expect(failed.title == "Backend discovery failed")
        #expect(failed.explanation == "Disconnected")
    }
}
