import SwiftUI

struct VolumeRenameView: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore
    @Environment(\.dismiss) private var dismiss
    @State private var label: String
    @State private var previewedLabel: String?
    @State private var isValidating = false
    @State private var errorMessage: String?

    init(node: DeviceNode, store: DiskStore) {
        self.node = node
        self.store = store
        _label = State(initialValue: node.device.volumeLabel ?? node.displayName)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(previewedLabel == nil ? "Rename Volume" : "Confirm Volume Rename").font(.title2).fontWeight(.semibold)
            Text("\(node.displayName) · \(node.device.identifier)").foregroundStyle(.secondary).textSelection(.enabled)
            if let previewedLabel {
                LabeledContent("Current label", value: node.device.volumeLabel ?? node.displayName)
                LabeledContent("New label", value: previewedLabel)
                LabeledContent("Current mount path", value: node.device.mountPoint ?? "Not reported")
                Text("macOS may change the volume’s path in /Volumes. Apps and shortcuts using the old path may need updating.")
                    .font(.callout).foregroundStyle(.secondary)
            } else {
                TextField("New volume label", text: $label).textFieldStyle(.roundedBorder).disabled(isValidating)
                Text("Diskvio will validate the name and current device safety before showing a confirmation.")
                    .font(.callout).foregroundStyle(.secondary)
            }
            if let errorMessage { Label(errorMessage, systemImage: "exclamationmark.triangle").font(.callout).textSelection(.enabled) }
            HStack {
                if isValidating { ProgressView().controlSize(.small) }
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }.keyboardShortcut(.cancelAction)
                if let previewedLabel {
                    Button("Back") { self.previewedLabel = nil; errorMessage = nil }
                    Button("Rename") {
                        dismiss()
                        Task { await store.perform(.renameVolume, on: node, volumeLabel: previewedLabel) }
                    }.disabled(store.isBusy)
                } else {
                    Button("Preview") { Task { await preview() } }
                        .disabled(isValidating || store.isBusy || label.isEmpty || label == node.device.volumeLabel)
                }
            }
        }
        .padding(24)
        .frame(width: 480)
        .interactiveDismissDisabled(isValidating)
    }

    @MainActor private func preview() async {
        guard let identity = node.device.identityToken else { return }
        isValidating = true
        errorMessage = nil
        defer { isValidating = false }
        do {
            try await store.preview(OperationRequest(action: .renameVolume, identifier: node.device.identifier,
                expectedIdentity: identity, volumeLabel: label, expectedMountPoints: node.device.confirmedMountPoints))
            previewedLabel = label
        } catch { errorMessage = error.localizedDescription }
    }
}
