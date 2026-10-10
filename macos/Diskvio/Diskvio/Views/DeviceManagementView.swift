import SwiftUI

struct DeviceManagementView: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore
    @State private var capabilities: OperationCapabilities?
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Management").font(.headline)
            if node.hasManagementActions {
                ViewThatFits(in: .horizontal) {
                    HStack { DeviceActionButtons(node: node, store: store) }
                    VStack(alignment: .leading) { DeviceActionButtons(node: node, store: store) }
                }
            }
            if let reason = capabilities?.unsupportedReason {
                Label(reason, systemImage: "lock.shield").font(.callout).foregroundStyle(.secondary)
            }
            if let errorMessage { Text(errorMessage).font(.caption).foregroundStyle(.secondary) }
            ForEach(capabilities?.limitations ?? [], id: \.self) { limitation in
                Text(limitation).font(.caption).foregroundStyle(.secondary)
            }
        }
        .task(id: "\(node.id):\(node.device.identityToken ?? ""):\(node.device.actions ?? []):\(node.device.mountPoint ?? "")") {
            capabilities = nil
            errorMessage = nil
            do {
                let identifier = node.device.identifier
                let result = try await Task.detached { try DiskService.supportedOperations(identifier: identifier) }.value
                guard !Task.isCancelled else { return }
                capabilities = result
            } catch {
                guard !Task.isCancelled else { return }
                errorMessage = "Capabilities unavailable: \(error.localizedDescription)"
            }
        }
    }
}
