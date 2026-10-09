import Foundation
import Testing
@testable import Diskvio

struct DiskvioTests {
    @Test func decodesDiskResponse() throws {
        let data = Data(#"{"status":"ok","disks":[{"number":0,"name":"Example SSD","size_bytes":500000000000,"bus_type":"PCI-Express","partition_style":"GUID_partition_scheme"}]}"#.utf8)
        let disks = try DiskService.decodeResponse(data)

        #expect(disks.count == 1)
        #expect(disks[0].number == 0)
        #expect(disks[0].name == "Example SSD")
        #expect(disks[0].sizeBytes == 500_000_000_000)
        #expect(disks[0].busType == "PCI-Express")
        #expect(disks[0].partitionStyle == "GUID_partition_scheme")
    }

    @Test func reportsRustError() throws {
        let data = Data(#"{"status":"error","message":"diskutil failed"}"#.utf8)
        #expect(throws: DiskServiceError.self) {
            try DiskService.decodeResponse(data)
        }
    }
}

private nonisolated let topologyFixture = #"""
{"status":"ok","inventory":{"disks":[{"device":{"identifier":"disk2","name":"Test USB","size_bytes":1000},"number":2,"partition_scheme":"gpt","connection_type":"USB","internal":false,"removable":true,"ejectable":true,"partitions":[{"device":{"identifier":"disk2s1","name":"Store","size_bytes":900,"partition_uuid":"partition-uuid"},"content_type":"Apple_APFS","offset_bytes":100,"apfs_container_id":"disk3"}]}],"apfs_containers":[{"device":{"identifier":"disk3","name":"APFS Container disk3","size_bytes":900,"used_bytes":300,"available_bytes":600},"uuid":"container-uuid","physical_store_ids":["disk2s1"],"volumes":[{"device":{"identifier":"disk3s1","name":"Test Data","size_bytes":900,"volume_uuid":"volume-uuid","filesystem":{"name":"APFS","kind":"apfs"},"used_bytes":200,"available_bytes":600},"roles":["Data"],"locked":false,"encrypted":false}]}],"warnings":[]}}
"""#

struct ExplorerTests {
    @Test func topologyDoesNotTreatAPFSVolumesAsPartitions() throws {
        let inventory = try DiskService.decodeInventory(Data(topologyFixture.utf8))
        #expect(inventory.disks[0].partitions.count == 1)
        let nodes = inventory.nodes.flatMap(\.flattened)
        #expect(nodes.map(\.kind) == [.disk, .partition, .container, .volume])
        #expect(nodes[3].device.availableBytes == 600)
        #expect(nodes[3].uuid == "volume-uuid")
    }

    @Test @MainActor func refreshPreservesUUIDSelectionAcrossVolumeRename() async throws {
        let fixture = try DiskService.decodeInventory(Data(topologyFixture.utf8))
        let renamed = try DiskService.decodeInventory(Data(topologyFixture.replacingOccurrences(of: "Test Data", with: "Renamed Data").utf8))
        let store = DiskStore(loadInventory: { fixture })
        await store.refresh()
        let volumeID = try #require(store.nodes.flatMap(\.flattened).last?.id)
        store.selection = volumeID
        await store.refresh()
        #expect(store.selection == volumeID)
        #expect(renamed.nodes.flatMap(\.flattened).last?.id == volumeID)
    }

    @Test @MainActor func removalImmediatelyClearsSelectedDiskHierarchy() async throws {
        let fixture = try DiskService.decodeInventory(Data(topologyFixture.utf8))
        let store = DiskStore(loadInventory: { fixture })
        await store.refresh()
        store.selection = store.nodes.flatMap(\.flattened).last?.id
        store.receive(DiskEvent(kind: .disappeared, identifier: "disk2", wholeIdentifier: "disk2", physicalCandidate: true))
        #expect(store.selection == nil)
        #expect(store.inventory.disks.isEmpty)
        #expect(store.inventory.apfsContainers.isEmpty)
        store.stopMonitoring()
    }

    @Test func reportsOperationFailureAndRejectsIncompleteSuccess() {
        #expect(throws: DiskServiceError.self) {
            try DiskService.decodeOperation(Data(#"{"status":"error","message":"Volume is busy"}"#.utf8))
        }
        #expect(throws: DiskServiceError.self) {
            try DiskService.decodeOperation(Data(#"{"status":"ok"}"#.utf8))
        }
    }

    @Test func capacityHandlesUnknownAndLargeValues() {
        #expect(Capacity.string(nil) == "Not available")
        #expect(!Capacity.string(UInt64.max).isEmpty)
    }
}

import Synchronization

struct NotificationTests {
    @Test @MainActor func initialNotificationsDoNotScanAndBurstsAreCoalesced() async throws {
        let fixture = try DiskService.decodeInventory(Data(topologyFixture.utf8))
        let loads = Mutex(0)
        let store = DiskStore(loadInventory: {
            loads.withLock { $0 += 1 }
            return fixture
        })
        defer { store.stopMonitoring() }
        await store.refresh()
        store.receive(DiskEvent(kind: .appeared, identifier: "disk2", wholeIdentifier: "disk2", physicalCandidate: true))
        try await Task.sleep(for: .milliseconds(600))
        #expect(loads.withLock { $0 } == 1)
        for _ in 0..<20 {
            store.receive(DiskEvent(kind: .appeared, identifier: "disk9", wholeIdentifier: "disk9", physicalCandidate: true))
        }
        try await Task.sleep(for: .milliseconds(700))
        #expect(loads.withLock { $0 } == 2)
    }
}

struct PartitionMapTests {
    @Test func fallbackPositionIncludesReportedLeadingGap() throws {
        var disk = try DiskService.decodeInventory(Data(topologyFixture.utf8)).disks[0]
        disk.partitions.append(try JSONDecoder().decode(DiskPartition.self, from: Data(#"{"device":{"identifier":"disk2s2","name":"Next","size_bytes":200}}"#.utf8)))
        let layout = PartitionMapLayout(disk: disk)
        #expect(layout.segments[0].startBytes == 100)
        #expect(layout.segments[1].startBytes == 1000)
        #expect(layout.segments[1].lengthFraction == 0)
    }

    @Test func outOfBoundsAndUnknownSizesNeverCreateInventedCapacity() throws {
        let fixture = topologyFixture.replacingOccurrences(of: "\"offset_bytes\":100", with: "\"offset_bytes\":900")
        var disk = try DiskService.decodeInventory(Data(fixture.utf8)).disks[0]
        disk.partitions.append(try JSONDecoder().decode(DiskPartition.self, from: Data(#"{"device":{"identifier":"disk2s2","name":"Unknown","size_bytes":null},"offset_bytes":100}"#.utf8)))
        let layout = PartitionMapLayout(disk: disk)
        #expect(layout.segments[0].startFraction == 0.9)
        #expect(layout.segments[0].lengthFraction == 0.1)
        #expect(layout.segments[1].partition.device.sizeBytes == nil)
        #expect(layout.segments[1].lengthFraction == 0)
        #expect(layout.segments.count == 2)
    }

    @Test func overflowingPartitionEndStaysBounded() throws {
        let fixture = topologyFixture.replacingOccurrences(of: "\"offset_bytes\":100", with: "\"offset_bytes\":18446744073709551610")
        var disk = try DiskService.decodeInventory(Data(fixture.utf8)).disks[0]
        disk.partitions.append(try JSONDecoder().decode(DiskPartition.self, from: Data(#"{"device":{"identifier":"disk2s2","name":"Next","size_bytes":200}}"#.utf8)))
        let layout = PartitionMapLayout(disk: disk)
        #expect(layout.segments[1].startBytes == UInt64.max)
        #expect(layout.segments.allSatisfy { $0.startFraction == 1 && $0.lengthFraction == 0 })
    }
}

struct OperationContractTests {
    @Test func queryRequestsCarryExplicitModes() throws {
        let query = OperationQueryRequest(mode: "supported_operations", action: nil, identifier: "disk2s1", expectedIdentity: nil)
        let json = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(query)) as? [String: Any])
        #expect(json["mode"] as? String == "supported_operations")
        #expect(json["action"] == nil)
        let legacy = OperationRequest(action: .mount, identifier: "disk2s1", expectedIdentity: "token")
        let legacyJSON = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(legacy)) as? [String: Any])
        #expect(legacyJSON["mode"] == nil)
        #expect(legacyJSON["expected_identity"] as? String == "token")
    }

    @Test func decodesCapabilitiesAndValidation() throws {
        let capabilities = try DiskService.decodeCapabilities(Data(#"{"status":"ok","capabilities":{"identifier":"disk3","device_kind":"apfs_container","identity_token":null,"actions":[]}}"#.utf8))
        #expect(capabilities.actions.isEmpty)
        #expect(capabilities.deviceKind == "apfs_container")
        let validation = try DiskService.decodeValidation(Data(#"{"status":"ok","validation":{"valid":true,"action":"mount","identifier":"disk2s1","expected_identity":"token"}}"#.utf8))
        #expect(validation.valid)
        #expect(validation.action == .mount)
    }

    @Test func preservesStructuredOperationErrors() throws {
        let data = Data(#"{"status":"error","message":"Access denied","error":{"code":"permission_denied","message":"Access denied","platform_code":5}}"#.utf8)
        do {
            _ = try DiskService.decodeOperation(data)
            Issue.record("Expected a structured operation error")
        } catch DiskServiceError.operationFailed(let error) {
            #expect(error.code == "permission_denied")
            #expect(error.platformCode == 5)
        }
    }
}
