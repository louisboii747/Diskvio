import SwiftUI

struct DeviceInspector: View {
    let node: DeviceNode

    var body: some View {
        Form {
            Section("Device") {
                PropertyRow(title: "Type", value: node.kind.rawValue)
                PropertyRow(title: "Identifier", value: node.device.identifier)
                PropertyRow(title: "Name", value: node.device.name)
                PropertyRow(title: "Media name", value: node.device.mediaName ?? node.disk.device.mediaName)
                PropertyRow(title: node.kind == .volume ? node.capacityLabel : "Total capacity", value: Capacity.string(node.device.sizeBytes))
                PropertyRow(title: "Filesystem", value: node.device.filesystem?.name)
                if node.kind == .partition { PropertyRow(title: "Partition type", value: node.contentType) }
                PropertyRow(title: "Partition scheme", value: node.disk.schemeName)
                PropertyRow(title: "Connection", value: node.disk.connectionType)
                PropertyRow(title: "Location", value: node.disk.internal.map { $0 ? "Internal" : "External" })
                PropertyRow(title: "Removable media", value: yesNo(node.disk.removable))
                PropertyRow(title: "Ejectable", value: yesNo(node.disk.ejectable))
            }
            Section("Volume & Storage") {
                PropertyRow(title: "Mount point", value: node.device.mountPoint ?? (node.device.filesystem == nil ? "Not applicable" : "Not mounted directly"))
                if node.kind == .container {
                    PropertyRow(title: "Container UUID", value: node.container?.uuid)
                } else {
                    PropertyRow(title: "Volume UUID", value: node.device.volumeUUID)
                    PropertyRow(title: "Partition UUID", value: node.device.partitionUUID)
                }
                PropertyRow(title: "Used", value: Capacity.string(node.device.usedBytes))
                PropertyRow(title: node.kind == .volume ? "Available to volume" : "Available", value: Capacity.string(node.device.availableBytes))
                if let volume = node.volume {
                    PropertyRow(title: "Roles", value: volume.roles.isEmpty ? "No special role" : volume.roles.joined(separator: ", "))
                    PropertyRow(title: "Encrypted", value: yesNo(volume.encrypted))
                    PropertyRow(title: "Locked", value: yesNo(volume.locked))
                    PropertyRow(title: "Quota", value: volume.quotaBytes.map { Capacity.string($0) } ?? "No quota")
                    PropertyRow(title: "Reserve", value: volume.reserveBytes.map { Capacity.string($0) } ?? "No reserve")
                    ForEach(volume.mountedSnapshots ?? [], id: \.identifier) { snapshot in
                        PropertyRow(title: "Mounted snapshot", value: snapshot.identifier)
                        PropertyRow(title: "Snapshot mount point", value: snapshot.mountPoint)
                        PropertyRow(title: "Snapshot UUID", value: snapshot.uuid)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    private func yesNo(_ value: Bool?) -> String? { value.map { $0 ? "Yes" : "No" } }
}

private struct PropertyRow: View {
    let title: String
    let value: String?
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            Text(value ?? "Not available")
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(title)
        .accessibilityValue(value ?? "Not available")
    }
}
