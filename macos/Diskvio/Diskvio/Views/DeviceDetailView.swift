import SwiftUI

struct DeviceDetailView: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                overview
                if let total = node.device.sizeBytes, let used = node.device.usedBytes, total > 0 {
                    VStack(alignment: .leading, spacing: 6) {
                        ProgressView(value: min(Double(used) / Double(total), 1))
                            .accessibilityLabel("Storage used").accessibilityValue(Capacity.string(used))
                        Text("\(Capacity.string(used)) used of \(Capacity.string(total))")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
                Divider()
                PartitionMapView(disk: node.disk, selectedIdentifier: node.physicalPartitionIdentifier ?? node.device.identifier, store: store) { identifier in
                    store.selectPartition(identifier: identifier, diskIdentifier: node.disk.device.identifier)
                }
                if node.kind == .volume {
                    Label("APFS volumes share container capacity. Available space is shared, rather than unallocated disk space.", systemImage: "square.stack.3d.up")
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
                if !relatedNodes.isEmpty {
                    relatedDevices
                }
                Divider()
                DeviceManagementView(node: node, store: store)
                if node.kind == .container, let container = node.container {
                    LabeledContent("Physical stores", value: container.physicalStoreIDs.joined(separator: ", "))
                        .font(.callout).foregroundStyle(.secondary).textSelection(.enabled)
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var overview: some View {
        VStack(alignment: .leading, spacing: 18) {
            if node.kind != .disk {
                Button { store.selectPhysicalDisk() } label: {
                    Label(node.disk.device.name, systemImage: "chevron.left")
                        .lineLimit(1)
                }.buttonStyle(.link).help("Inspect the physical disk")
            }
            HStack(alignment: .top, spacing: 14) {
                Image(systemName: node.deviceSymbol)
                    .font(.system(size: 36, weight: .light)).foregroundStyle(.secondary)
                    .frame(width: 44, height: 48).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 5) {
                    Text(node.displayName).font(.title2.weight(.semibold)).textSelection(.enabled)
                    Text("\(node.kind.rawValue) · \(node.device.identifier)")
                        .font(.callout).foregroundStyle(.secondary).textSelection(.enabled)
                    if node.kind == .disk, let model = node.disk.model ?? node.device.mediaName, model != node.displayName {
                        Text(model).font(.callout).foregroundStyle(.secondary).textSelection(.enabled)
                    }
                }
                Spacer(minLength: 0)
            }
            if !node.badges.isEmpty {
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 6) { badges }
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 100), alignment: .leading)], alignment: .leading, spacing: 6) { badges }
                }
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 120), alignment: .leading)], alignment: .leading, spacing: 14) {
                CapacitySummary(label: node.capacityLabel, bytes: node.device.sizeBytes)
                if let used = node.device.usedBytes { CapacitySummary(label: "Used", bytes: used) }
                if let free = node.device.availableBytes {
                    CapacitySummary(label: node.kind == .volume ? "Available to volume" : "Available", bytes: free)
                }
            }
            Grid(alignment: .leading, horizontalSpacing: 20, verticalSpacing: 8) {
                GridRow {
                    Text("Connection").foregroundStyle(.secondary)
                    Text(node.connectionDescription)
                }
                GridRow {
                    Text("Partition map").foregroundStyle(.secondary)
                    Text(node.disk.schemeName)
                }
                GridRow {
                    Text("Status").foregroundStyle(.secondary)
                    Text(node.reportedStatus)
                }
            }.font(.callout).textSelection(.enabled)
        }
    }

    private var badges: some View {
        ForEach(node.badges, id: \.self) { badge in
            Text(badge).font(.caption).foregroundStyle(.secondary)
                .padding(.horizontal, 8).padding(.vertical, 4)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
        }
    }

    private var relatedNodes: [DeviceNode] {
        if node.kind == .disk { return node.flattened.filter { $0.kind == .volume } }
        return node.children ?? []
    }

    private var relatedDevices: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(node.kind == .partition ? "Container" : "Volumes").font(.headline)
            ForEach(relatedNodes) { child in
                Button { store.selection = child.id } label: {
                    HStack(spacing: 10) {
                        Image(systemName: child.deviceSymbol).foregroundStyle(.secondary).frame(width: 22)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(child.displayName).fontWeight(.medium).lineLimit(2).multilineTextAlignment(.leading)
                            Text(child.device.mountPoint ?? child.device.identifier).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                        Spacer(minLength: 8)
                        Text(child.capacityDescription).font(.callout).monospacedDigit().foregroundStyle(.secondary)
                            .multilineTextAlignment(.trailing)
                        Image(systemName: "chevron.right").font(.caption).foregroundStyle(.tertiary)
                    }
                    .padding(10).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .background(store.selection == child.id ? Color.accentColor.opacity(0.12) : .clear, in: RoundedRectangle(cornerRadius: 6))
                .contextMenu { DeviceContextMenu(node: child, store: store) }
                .help("Inspect \(child.displayName)")
            }
        }
    }
}

private struct CapacitySummary: View {
    let label: String
    let bytes: UInt64?
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(Capacity.string(bytes)).font(.title3.weight(.medium)).monospacedDigit().textSelection(.enabled)
        }
        .help(bytes.map { "\($0.formatted()) bytes" } ?? "Capacity not reported")
    }
}
