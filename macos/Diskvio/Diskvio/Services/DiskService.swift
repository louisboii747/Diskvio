import DiskvioFFI
import Foundation

enum DiskServiceError: LocalizedError {
    case missingResponse
    case invalidUTF8
    case invalidResponse
    case discoveryFailed(String)

    var errorDescription: String? {
        switch self {
        case .missingResponse:
            "Rust could not return a response."
        case .invalidUTF8:
            "Rust returned invalid UTF-8."
        case .invalidResponse:
            "Rust returned an incomplete response."
        case .discoveryFailed(let message):
            message
        }
    }
}

struct DiskService: Sendable {
    nonisolated static func listDisks() throws -> [Disk] {
        guard let pointer = diskvio_list_disks_json() else {
            throw DiskServiceError.missingResponse
        }
        defer { diskvio_string_free(pointer) }

        guard let json = String(validatingCString: pointer) else {
            throw DiskServiceError.invalidUTF8
        }
        return try decodeResponse(Data(json.utf8))
    }

    nonisolated static func decodeResponse(_ data: Data) throws -> [Disk] {
        let response = try JSONDecoder().decode(Response.self, from: data)
        switch response.status {
        case .ok:
            guard let disks = response.disks else {
                throw DiskServiceError.invalidResponse
            }
            return disks
        case .error:
            throw DiskServiceError.discoveryFailed(
                response.message ?? "Disk discovery failed without an error message."
            )
        }
    }

    nonisolated static func inventory() throws -> DiskInventory {
        guard let pointer = diskvio_inventory_json() else { throw DiskServiceError.missingResponse }
        defer { diskvio_string_free(pointer) }
        guard let json = String(validatingCString: pointer) else { throw DiskServiceError.invalidUTF8 }
        return try decodeInventory(Data(json.utf8))
    }

    nonisolated static func decodeInventory(_ data: Data) throws -> DiskInventory {
        let response = try JSONDecoder().decode(InventoryResponse.self, from: data)
        guard response.status == .ok else {
            throw DiskServiceError.discoveryFailed(response.message ?? "Disk discovery failed.")
        }
        guard let inventory = response.inventory else { throw DiskServiceError.invalidResponse }
        return inventory
    }

    nonisolated static func perform(_ request: OperationRequest) throws -> OperationOutcome {
        let data = try JSONEncoder().encode(request)
        return try data.withUnsafeBytes { buffer in
            guard let pointer = diskvio_operation_json(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count) else {
                throw DiskServiceError.missingResponse
            }
            defer { diskvio_string_free(pointer) }
            guard let json = String(validatingCString: pointer) else { throw DiskServiceError.invalidUTF8 }
            return try decodeOperation(Data(json.utf8))
        }
    }

    nonisolated static func decodeOperation(_ data: Data) throws -> OperationOutcome {
        let response = try JSONDecoder().decode(OperationResponse.self, from: data)
        guard response.status == .ok else {
            throw DiskServiceError.discoveryFailed(response.message ?? "The operation failed.")
        }
        guard let operation = response.operation else { throw DiskServiceError.invalidResponse }
        return operation
    }

    private nonisolated struct OperationResponse: Decodable {
        let status: Response.Status
        let operation: OperationOutcome?
        let message: String?
    }

    private nonisolated struct InventoryResponse: Decodable {
        let status: Response.Status
        let inventory: DiskInventory?
        let message: String?
    }

    private nonisolated struct Response: Decodable {
        enum Status: String, Decodable {
            case ok, error
        }

        let status: Status
        let disks: [Disk]?
        let message: String?
    }
}
