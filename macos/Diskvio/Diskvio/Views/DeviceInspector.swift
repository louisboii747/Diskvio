import AppKit
import SwiftUI

struct DeviceInspector: View {
    let node: DeviceNode
    let planning: PartitionPlanningPresentation

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Information").font(.headline)
                Spacer()
                Image(systemName: node.deviceSymbol).foregroundStyle(.secondary).accessibilityHidden(true)
            }.padding(16)
            Divider()
            Form {
                Section("General") {
                    PropertyRow(title: "Name", value: node.displayName)
                    PropertyRow(title: "Type", value: node.kind.rawValue)
                    PropertyRow(title: "Identifier", value: node.device.identifier, copyable: true)
                    PropertyRow(title: "Model", value: node.disk.model ?? node.disk.device.mediaName)
                    PropertyRow(title: "Manufacturer", value: node.disk.manufacturer)
                    PropertyRow(title: "Status", value: node.reportedStatus)
                    PropertyRow(title: "Health / SMART", value: node.device.healthStatus)
                }
                Section("Storage") {
                    PropertyRow(title: node.capacityLabel, value: Capacity.string(node.device.sizeBytes))
                    PropertyRow(title: "Exact capacity", value: StoragePresentation(node: node).exactCapacity, copyable: node.device.sizeBytes != nil)
                    PropertyRow(title: "Used", value: Capacity.string(node.device.usedBytes))
                    PropertyRow(title: node.kind == .volume ? "Available to volume" : "Available", value: Capacity.string(node.device.availableBytes))
                    PropertyRow(title: "Filesystem", value: node.device.filesystem?.name)
                    PropertyRow(title: "Partition scheme", value: node.disk.schemeName)
                    PropertyRow(title: "Partition count", value: String(node.disk.partitions.count))
                    PropertyRow(title: "Physical disk", value: node.disk.device.identifier, copyable: true)
                    if let partition = node.partition {
                        PropertyRow(title: "Partition number", value: partition.number.map(String.init))
                        PropertyRow(title: "Partition role", value: PartitionPresentation(partition: partition).typeName)
                        PropertyRow(title: "Partition type", value: node.contentType)
                        PropertyRow(title: "Offset", value: partition.offsetBytes.map { "\($0.formatted()) bytes" })
                        PropertyRow(title: "Bounds (exclusive end)", value: StoragePresentation(node: node).partitionBounds)
                    }
                }
                Section("Connection") {
                    PropertyRow(title: "Interface", value: node.disk.connectionType)
                    PropertyRow(title: "Location", value: node.disk.internal.map { $0 ? "Internal" : "External" })
                    PropertyRow(title: "Removable media", value: yesNo(node.disk.removable))
                    PropertyRow(title: "Ejectable", value: yesNo(node.disk.ejectable))
                    PropertyRow(title: "Mount point", value: node.device.mountPoint ?? (node.device.filesystem == nil ? "Not applicable" : "Not mounted directly"), copyable: node.device.mountPoint != nil)
                }
                Section("Protection") {
                    Text(StoragePresentation(node: node).protectionExplanation).font(.callout).foregroundStyle(.secondary)
                    PropertyRow(title: "System", value: yesNo(node.device.safety?.system))
                    PropertyRow(title: "Boot", value: yesNo(node.device.safety?.boot))
                    PropertyRow(title: "Recovery", value: yesNo(node.device.safety?.recovery))
                    PropertyRow(title: "Read-only", value: yesNo(node.device.safety?.readOnly))
                    PropertyRow(title: "Hidden", value: yesNo(node.device.safety?.hidden))
                    PropertyRow(title: "Offline", value: yesNo(node.device.safety?.offline))
                    if let volume = node.volume {
                        PropertyRow(title: "Roles", value: volume.roles.isEmpty ? "No special role" : volume.roles.joined(separator: ", "))
                        PropertyRow(title: "Encrypted", value: yesNo(volume.encrypted))
                        PropertyRow(title: "Locked", value: yesNo(volume.locked))
                        PropertyRow(title: "Quota", value: volume.quotaBytes.map { Capacity.string($0) } ?? "No quota")
                        PropertyRow(title: "Reserve", value: volume.reserveBytes.map { Capacity.string($0) } ?? "No reserve")
                    }
                }
                Section("Advanced") {
                    PropertyRow(title: "Backend name", value: node.device.name)
                    PropertyRow(title: "Volume label", value: node.device.volumeLabel)
                    PropertyRow(title: "Media name", value: node.device.mediaName ?? node.disk.device.mediaName)
                    PropertyRow(title: "Stable identifier", value: node.device.stableID ?? node.disk.device.stableID, copyable: true)
                    PropertyRow(title: "Volume UUID", value: node.device.volumeUUID, copyable: true)
                    PropertyRow(title: "Partition UUID", value: node.device.partitionUUID, copyable: true)
                    if node.kind == .container { PropertyRow(title: "Container UUID", value: node.container?.uuid, copyable: true) }
                    ForEach(node.volume?.mountedSnapshots ?? [], id: \.identifier) { snapshot in
                        PropertyRow(title: "Mounted snapshot", value: snapshot.identifier, copyable: true)
                        PropertyRow(title: "Snapshot mount point", value: snapshot.mountPoint, copyable: true)
                        PropertyRow(title: "Snapshot UUID", value: snapshot.uuid, copyable: true)
                    }
                    DisclosureGroup("Partition planning") {
                        Text(planning.title).font(.callout)
                        Text(planning.explanation).font(.caption).foregroundStyle(.secondary)
                        if !planning.technicalDetails.isEmpty {
                            Text(planning.technicalDetails).font(.caption).textSelection(.enabled)
                        }
                    }
                }
            }.formStyle(.grouped)
        }
    }

    private func yesNo(_ value: Bool?) -> String? { value.map { $0 ? "Yes" : "No" } }
}

private struct PropertyRow: View {
    let title: String
    let value: String?
    var copyable = false

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.caption).foregroundStyle(.secondary)
                Text(value ?? "Not available").textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }.frame(maxWidth: .infinity, alignment: .leading)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(title).accessibilityValue(value ?? "Not available")
            if copyable, let value {
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(value, forType: .string)
                } label: { Image(systemName: "doc.on.doc") }
                .buttonStyle(.borderless).foregroundStyle(.secondary)
                .help("Copy \(title.lowercased())").accessibilityLabel("Copy \(title.lowercased())")
            }
        }.padding(.vertical, 2)
    }
}
