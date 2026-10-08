import SwiftUI

struct DeviceSidebar: View {
    @ObservedObject var store: DiskStore

    var body: some View {
        List(selection: $store.selection) {
            Section("Storage Devices") {
                OutlineGroup(store.nodes, children: \.children) { node in
                    DeviceRow(node: node, isSelected: store.selection == node.id)
                        .tag(node.id)
                        .contextMenu { DeviceContextMenu(node: node, store: store) }
                }
            }
        }
        .listStyle(.sidebar)
        .overlay {
            if store.nodes.isEmpty && store.isLoading { ProgressView("Discovering disks…") }
        }
        .safeAreaInset(edge: .bottom) {
            HStack(spacing: 6) {
                if store.isLoading { ProgressView().controlSize(.small) }
                Text(store.isLoading ? "Refreshing…" : "\(store.inventory.disks.count) physical \(store.inventory.disks.count == 1 ? "disk" : "disks")")
                Spacer(minLength: 0)
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding(12)
        }
    }
}

struct DeviceRow: View {
    let node: DeviceNode
    let isSelected: Bool
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: node.kind.symbol)
                .frame(width: 18)
                .foregroundColor(isSelected ? nil : .secondary)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(node.device.name).lineLimit(1).truncationMode(.tail)
                ViewThatFits(in: .horizontal) {
                    Text("\(node.kind.rawValue) · \(node.capacityDescription)")
                        .fixedSize(horizontal: true, vertical: false)
                    Text(node.capacityDescription).lineLimit(1).truncationMode(.tail)
                }
                .font(.caption).foregroundColor(isSelected ? nil : .secondary)
            }
            .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 3)
        .help("\(node.device.name) — \(node.device.identifier)\n\(node.kind.rawValue) · \(node.capacityDescription)")
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(node.device.name), \(node.kind.rawValue), \(node.device.identifier), \(node.capacityDescription)")
    }
}
