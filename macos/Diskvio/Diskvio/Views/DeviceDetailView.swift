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
                        Text(node.displayName).font(.title2).fontWeight(.semibold).textSelection(.enabled)
                        Text("\(node.kind.rawValue) · \(node.device.identifier)")
                            .foregroundStyle(.secondary).textSelection(.enabled)
                    }
                    Spacer(minLength: 0)
                }
                if !node.badges.isEmpty {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 90), alignment: .leading)], alignment: .leading, spacing: 6) {
                        ForEach(node.badges, id: \.self) { badge in
                            Text(badge).font(.caption).foregroundStyle(.secondary)
                                .padding(.horizontal, 8).padding(.vertical, 4)
                                .background(.quaternary, in: Capsule())
                        }
                    }
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), alignment: .leading)], alignment: .leading, spacing: 12) {
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
                if !node.disk.partitions.isEmpty {
                    PartitionMapView(disk: node.disk, selectedIdentifier: node.physicalPartitionIdentifier ?? node.device.identifier, store: store) { identifier in
                        store.selectPartition(identifier: identifier, diskIdentifier: node.disk.device.identifier)
                    }
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
                if node.kind == .disk {
                    VStack(alignment: .leading, spacing: 8) {
                        LabeledContent("Physical disk", value: "Disk \(node.disk.number) · \(node.disk.device.identifier)")
                        LabeledContent("Partition map", value: node.disk.schemeName)
                        LabeledContent("Connection", value: node.disk.connectionType ?? "Not available")
                        LabeledContent("Partitions", value: String(node.disk.partitions.count))
                        LabeledContent("Read-only", value: node.disk.device.safety?.readOnly.map { $0 ? "Yes" : "No" } ?? "Unknown")
                    }.font(.callout).textSelection(.enabled)
                } else {
                    Button { store.selectPhysicalDisk() } label: {
                        Label("Inspect physical disk \(node.disk.number)", systemImage: "externaldrive")
                    }.help("Select the parent disk in the sidebar and inspector")
                }
                VStack(alignment: .leading, spacing: 6) {
                    Label(store.planningPresentation.title, systemImage: "rectangle.split.3x1")
                        .font(.callout)
                    Text(store.planningPresentation.explanation).font(.caption).foregroundStyle(.secondary)
                }
                Divider()
                DeviceManagementView(node: node, store: store)
                Divider()
                if let children = node.children {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(childHeading).font(.headline)
                        ForEach(children) { child in
                            Button { store.selection = child.id } label: {
                                HStack(spacing: 10) {
                                    Image(systemName: child.kind.symbol).foregroundStyle(.secondary).frame(width: 22)
                                    VStack(alignment: .leading, spacing: 3) {
                                        Text(child.displayName).fontWeight(.medium).lineLimit(2).multilineTextAlignment(.leading)
                                        Text(child.device.filesystem?.name ?? child.kind.rawValue).font(.caption).foregroundStyle(.secondary)
                                    }
                                    Spacer(minLength: 8)
                                    Text(child.capacityDescription).font(.callout).monospacedDigit().foregroundStyle(.secondary)
                                        .fixedSize(horizontal: false, vertical: true).multilineTextAlignment(.trailing)
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
