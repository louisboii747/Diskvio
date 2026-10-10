import Foundation
import Testing
@testable import DiskvioPresentation

private func partition(_ json: String) throws -> DiskPartition {
    try JSONDecoder().decode(DiskPartition.self, from: Data(json.utf8))
}
private let fixture = #"""
{"disks":[{"device":{"identifier":"disk2","name":"USB","stable_id":"iomedia:42","size_bytes":1000},"number":2,"partition_scheme":"gpt","internal":false,"removable":true,"connection_type":"USB","partitions":[{"device":{"identifier":"disk2s1","name":"disk2s1","size_bytes":1},"number":1,"content_type":"EFI"},{"device":{"identifier":"disk2s2","name":"disk2s2","size_bytes":999,"partition_uuid":"partition"},"number":2,"content_type":"Apple_APFS","apfs_container_id":"disk3"}]}],"apfs_containers":[{"device":{"identifier":"disk3","name":"APFS Container disk3","size_bytes":999},"uuid":"container","physical_store_ids":["disk2s2"],"volumes":[{"device":{"identifier":"disk3s1","name":"Photos","volume_label":"Photos","volume_uuid":"volume","size_bytes":999,"filesystem":{"name":"APFS","kind":"apfs"},"mount_point":"/Volumes/Photos","mount_points":["/Volumes/Photos"],"identity_token":"token","actions":["unmount","rename_volume"],"safety":{"read_only":false}},"roles":[],"locked":false,"encrypted":false}]}],"warnings":[]}
"""#
private func inventory(_ json: String = fixture) throws -> DiskInventory {
    try JSONDecoder().decode(DiskInventory.self, from: Data(json.utf8))
}

struct PresentationTests {
    @Test func actionGatingRequiresBackendCapabilityAndIdentity() throws {
        let volume = try #require(inventory().nodes.flatMap(\.flattened).last)
        #expect(volume.operationTarget(for: .renameVolume)?.id == volume.id)
        #expect(volume.operationTarget(for: .mount) == nil)
        #expect(volume.operationTarget(for: .setDriveLetter) == nil)
        let missing = try #require(inventory(fixture.replacingOccurrences(of: "\"identity_token\":\"token\",", with: "")).nodes.flatMap(\.flattened).last)
        #expect(missing.operationTarget(for: .renameVolume) == nil)
        #expect(!missing.hasManagementActions)
    }
    @Test func labelWinsOverRoleAndIdentifier() throws {
        let value = try partition(#"{"device":{"identifier":"disk2s1","name":"disk2s1","volume_label":"Photos"},"role":"basic_data"}"#)
        #expect(PartitionPresentation(partition: value).name == "Photos")
    }
    @Test(arguments: [
        ("EFI", "EFI System Partition", PartitionCategory.system),
        ("Apple_Boot", "Recovery Partition", .recovery),
        ("Apple_Recovery", "Recovery Partition", .recovery),
        ("Apple_APFS", "APFS Physical Store", .data),
        ("Apple_HFS", "Apple HFS Partition", .data),
        ("Apple_partition_map", "Apple Partition Map", .metadata),
        ("SomethingNew", "Partition 2", .unknown)
    ]) func knownTypesHaveFriendlyNamesAndCategories(content: String, name: String, category: PartitionCategory) throws {
        let value = try partition("{\"device\":{\"identifier\":\"disk2s2\",\"name\":\"disk2s2\"},\"number\":2,\"content_type\":\"\(content)\"}")
        #expect(PartitionPresentation(partition: value).name == name)
        #expect(PartitionPresentation(partition: value).category == category)
    }
    @Test func legacyFilesystemLabelsStillWork() throws {
        let value = try partition(#"{"device":{"identifier":"disk2s1","name":"My Archive","filesystem":{"name":"ExFAT","kind":"exfat"}}}"#)
        #expect(PartitionPresentation(partition: value).name == "My Archive")
    }
    @Test func labelDoesNotOverrideProtectionColour() throws {
        let value = try partition(#"{"device":{"identifier":"disk2s1","name":"Friendly","volume_label":"Friendly","safety":{"recovery":true}},"role":"recovery"}"#)
        #expect(PartitionPresentation(partition: value).category == .recovery)
    }
    @Test func tinyPartitionsStayVisibleAndWidthsConserveAvailableSpace() throws {
        let layout = PartitionMapLayout(disk: try inventory().disks[0])
        let widths = layout.visualWidths(availableWidth: 500)
        #expect(widths.allSatisfy { $0 >= 48 })
        #expect(widths[1] > widths[0])
        #expect(abs(widths.reduce(0, +) + 4 - 500) < 0.0001)
        #expect(layout.visualWidths(availableWidth: 60) == [48, 48])
    }
    @Test func unknownCapacityIsNotFabricated() throws {
        let unknown = fixture.replacingOccurrences(of: "\"size_bytes\":1}", with: "\"size_bytes\":null}")
        let layout = PartitionMapLayout(disk: try inventory(unknown).disks[0])
        #expect(layout.segments[0].partition.device.sizeBytes == nil)
        #expect(layout.segments.count == 2)
        #expect(layout.visualWidths(availableWidth: 500)[0] == 48)
    }
    @Test func apfsSelectionHighlightsPhysicalStoreWithoutExtraSegments() throws {
        let data = try inventory()
        let volume = try #require(data.nodes.flatMap(\.flattened).last)
        #expect(volume.physicalPartitionIdentifier == "disk2s2")
        #expect(volume.capacityLabel == "Shared capacity")
        #expect(PartitionMapLayout(disk: data.disks[0]).segments.count == 2)
        #expect(volume.displayName == "Photos")
    }
    @Test func selectionIdentitySurvivesRenameAndBsdNumberChange() throws {
        let old = try inventory().nodes.flatMap(\.flattened).last
        let changed = try inventory(fixture.replacingOccurrences(of: "Photos", with: "Archive").replacingOccurrences(of: "disk2", with: "disk8")).nodes.flatMap(\.flattened).last
        #expect(old?.id == changed?.id)
        #expect(changed?.displayName == "Archive")
    }
    @Test func badgesUseOnlyReportedMetadata() throws {
        let volume = try #require(inventory().nodes.flatMap(\.flattened).last)
        #expect(volume.badges == ["External", "Removable", "USB"])
        #expect(!volume.badges.contains("System"))
        #expect(!volume.badges.contains("Read-only"))
    }
    @Test func renameContractPreservesExactLabelIdentityAndMountSnapshot() throws {
        let request = OperationRequest(action: .renameVolume, identifier: "disk3s1", expectedIdentity: "token", volumeLabel: "Photos — été", expectedMountPoints: ["/Volumes/Photos"])
        let json = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(request)) as? [String: Any])
        #expect(json["action"] as? String == "rename_volume")
        #expect(json["volume_label"] as? String == "Photos — été")
        #expect(json["expected_mount_points"] as? [String] == ["/Volumes/Photos"])
    }
    @Test func extendedAndLegacyCapabilitiesDecode() throws {
        let extended = try JSONDecoder().decode(OperationCapabilities.self, from: Data(#"{"identifier":"disk2","device_kind":"physical_disk","actions":[],"unsupported_reason":"Protected disk","limitations":["Create is unsupported"],"label_max_length":63}"#.utf8))
        #expect(extended.unsupportedReason == "Protected disk")
        #expect(extended.labelMaxLength == 63)
        let legacy = try JSONDecoder().decode(OperationCapabilities.self, from: Data(#"{"identifier":"disk2","device_kind":"physical_disk","actions":[]}"#.utf8))
        #expect(legacy.limitations == nil)
    }
}
