import DiskvioFFI
import Foundation

enum DiskServiceError: LocalizedError {
    case missingResponse
    case invalidUTF8
    case invalidResponse
    case discoveryFailed(String)
    case operationFailed(BackendOperationError)

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
        case .operationFailed(let error):
            error.message
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
        try decodeOperation(operationResponseData(request))
    }

    nonisolated static func supportedOperations(identifier: String) throws -> OperationCapabilities {
        let request = OperationQueryRequest(mode: "supported_operations", action: nil, identifier: identifier, expectedIdentity: nil)
        return try decodeCapabilities(operationResponseData(request))
    }

    nonisolated static func validate(_ request: OperationRequest) throws -> OperationValidation {
        let query = OperationQueryRequest(mode: "validate", action: request.action, identifier: request.identifier, expectedIdentity: request.expectedIdentity)
        return try decodeValidation(operationResponseData(query))
    }

    private nonisolated static func operationResponseData(_ request: some Encodable) throws -> Data {
        let data = try JSONEncoder().encode(request)
        return try data.withUnsafeBytes { buffer in
            guard let pointer = diskvio_operation_json(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count) else {
                throw DiskServiceError.missingResponse
            }
            defer { diskvio_string_free(pointer) }
            guard let json = String(validatingCString: pointer) else { throw DiskServiceError.invalidUTF8 }
            return Data(json.utf8)
        }
    }

    nonisolated static func decodeCapabilities(_ data: Data) throws -> OperationCapabilities {
        let response = try JSONDecoder().decode(OperationResponse.self, from: data)
        try checkOperationResponse(response)
        guard let capabilities = response.capabilities else { throw DiskServiceError.invalidResponse }
        return capabilities
    }

    nonisolated static func decodeValidation(_ data: Data) throws -> OperationValidation {
        let response = try JSONDecoder().decode(OperationResponse.self, from: data)
        try checkOperationResponse(response)
        guard let validation = response.validation, validation.valid else { throw DiskServiceError.invalidResponse }
        return validation
    }

    private nonisolated static func checkOperationResponse(_ response: OperationResponse) throws {
        guard response.status == .ok else {
            if let error = response.error { throw DiskServiceError.operationFailed(error) }
            throw DiskServiceError.discoveryFailed(response.message ?? "The operation failed.")
        }
    }

    nonisolated static func decodeOperation(_ data: Data) throws -> OperationOutcome {
        let response = try JSONDecoder().decode(OperationResponse.self, from: data)
        try checkOperationResponse(response)
        guard let operation = response.operation else { throw DiskServiceError.invalidResponse }
        return operation
    }

    private nonisolated struct OperationResponse: Decodable {
        let status: Response.Status
        let operation: OperationOutcome?
        let capabilities: OperationCapabilities?
        let validation: OperationValidation?
        let error: BackendOperationError?
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
