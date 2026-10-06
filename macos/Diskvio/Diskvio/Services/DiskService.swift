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
            "Rust could not return a disk discovery response."
        case .invalidUTF8:
            "Rust returned invalid UTF-8."
        case .invalidResponse:
            "Rust returned an incomplete disk discovery response."
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

    private struct Response: Decodable {
        enum Status: String, Decodable {
            case ok, error
        }

        let status: Status
        let disks: [Disk]?
        let message: String?
    }
}
