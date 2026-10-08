import SwiftUI

struct DeviceDetailView: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                HStack(alignment: .center, spacing: 14) {
                    Image(systemName: node.kind.symbol)
                        .font(.system(size: 34, weight: .regular))
                        .foregroundStyle(.secondary)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 5) {
                        Text(node.device.name).font(.title2).fontWeight(.semibold).textSelection(.enabled)
                        Text("\(node.kind.rawValue) · \(node.device.identifier)")
                            .foregroundStyle(.secondary).textSelection(.enabled)
                    }
                    Spacer(minLength: 0)
                }
                HStack(alignment: .top, spacing: 24) {
                    CapacitySummary(label: node.capacityLabel, bytes: node.device.sizeBytes)
                    if let used = node.device.usedBytes {
                        CapacitySummary(label: "Used", bytes: used)
                    }
                    if let available = node.device.availableBytes {
                        CapacitySummary(label: node.kind == .volume ? "Available to volume" : "Available", bytes: available)
                    }
                }
                if let total = node.device.sizeBytes, let used = node.device.usedBytes, total > 0 {
                    ProgressView(value: min(Double(used) / Double(total), 1))
                        .accessibilityLabel("Storage used")
                        .accessibilityValue(Capacity.string(used))
                }
                if node.kind == .volume {
                    Text("APFS volumes share the container’s free space. Used space reflects this volume’s allocation; snapshots and container metadata can consume additional space.")
                        .font(.callout).foregroundStyle(.secondary)
                    ForEach(node.volume?.mountedSnapshots ?? [], id: \.identifier) { snapshot in
                        if let path = snapshot.mountPoint {
                            Label("Mounted snapshot at \(path)", systemImage: "camera.aperture")
                                .font(.callout).foregroundStyle(.secondary)
                        }
                    }
                    if node.volume?.locked == true {
                        Label("This volume is locked. Unlock it in Disk Utility before mounting.", systemImage: "lock")
                            .font(.callout).foregroundStyle(.secondary)
                    }
                }
                if node.disk.internal == true {
                    Label("Internal disks are available for inspection only.", systemImage: "lock.shield")
                        .font(.callout).foregroundStyle(.secondary)
                }
                Divider()
                if node.kind == .disk || node.kind == .partition {
                    PartitionMapView(disk: node.disk, selectedIdentifier: node.device.identifier) { identifier in
                        store.selection = store.nodes.flatMap(\.flattened).first {
                            $0.disk.device.identifier == node.disk.device.identifier && $0.device.identifier == identifier
                        }?.id
                    }
                }
                if let children = node.children {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(childHeading).font(.headline)
                        ForEach(children) { child in
                            Button { store.selection = child.id } label: {
                                HStack(spacing: 10) {
                                    Image(systemName: child.kind.symbol).foregroundStyle(.secondary).frame(width: 22)
                                    VStack(alignment: .leading, spacing: 3) {
                                        Text(child.device.name).fontWeight(.medium)
                                        Text(child.device.filesystem?.name ?? child.kind.rawValue).font(.caption).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    Text(child.capacityDescription).monospacedDigit().foregroundStyle(.secondary)
                                    Image(systemName: "chevron.right").font(.caption).foregroundStyle(.tertiary)
                                }
                                .padding(.vertical, 7)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .contextMenu { DeviceContextMenu(node: child, store: store) }
                            if child.id != children.last?.id { Divider() }
                        }
                    }
                }
                if node.kind == .container, let container = node.container {
                    LabeledContent("Physical stores", value: container.physicalStoreIDs.joined(separator: ", "))
                        .font(.callout).foregroundStyle(.secondary)
                }
                if let lastUpdated = store.lastUpdated {
                    Text("Updated \(lastUpdated.formatted(date: .omitted, time: .standard))")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var childHeading: String {
        switch node.kind {
        case .disk: "Partitions"
        case .partition: "APFS Container"
        case .container: "Volumes"
        case .volume: "Volumes"
        }
    }
}

private struct CapacitySummary: View {
    let label: String
    let bytes: UInt64?
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(Capacity.string(bytes)).font(.title3).monospacedDigit().textSelection(.enabled)
        }
    }
}
