//
//  DiskvioTests.swift
//  DiskvioTests
//
//  Created by Louis Hinchliffe on 06/10/2026.
//

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
