import SwiftUI

struct DeviceSidebar: View {
    @ObservedObject var store: DiskStore
    @State private var search = ""
    @State private var filter = DeviceFilter.all

    private var visibleNodes: [DeviceNode] {
        store.nodes.filter { filter.includes($0.disk) && $0.matches(search) }
    }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 10) {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass").foregroundStyle(.secondary).accessibilityHidden(true)
                    TextField("Search devices and volumes", text: $search)
                        .textFieldStyle(.plain)
                        .accessibilityLabel("Search devices and volumes")
                    if !search.isEmpty {
                        Button { search = "" } label: { Image(systemName: "xmark.circle.fill") }
                            .buttonStyle(.plain).foregroundStyle(.secondary).help("Clear search")
                            .accessibilityLabel("Clear search")
                    }
                }
                .padding(7)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 6))
                Picker("Show", selection: $filter) {
                    ForEach(DeviceFilter.allCases) { filter in Text(filter.rawValue).tag(filter) }
                }
                .labelsHidden().frame(maxWidth: .infinity, alignment: .leading)
                .accessibilityLabel("Filter devices")
            }
            .padding(12)
            List(selection: $store.selection) {
                ForEach(DeviceGroup.allCases) { group in
                    let nodes = visibleNodes.filter { DeviceGroup.group(for: $0.disk) == group }
                    if !nodes.isEmpty {
                        Section(group.rawValue) {
                            if search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                                OutlineGroup(nodes, children: \.children) { node in
                                    sidebarRow(node)
                                }
                            } else {
                                ForEach(nodes.flatMap(\.flattened).filter { $0.matchesSelf(search) }) { node in
                                    sidebarRow(node)
                                }
                            }
                        }
                    }
                }
            }
            .listStyle(.sidebar)
            .overlay {
                if visibleNodes.isEmpty && !store.nodes.isEmpty {
                    ContentUnavailableView.search(text: search)
                } else if store.nodes.isEmpty && store.isLoading {
                    ProgressView("Discovering disks…")
                }
            }
            Divider()
            HStack(spacing: 7) {
                if store.isLoading { ProgressView().controlSize(.small) }
                else { Image(systemName: "externaldrive").accessibilityHidden(true) }
                Text(store.isLoading ? "Refreshing devices…" : "\(store.inventory.disks.count) physical \(store.inventory.disks.count == 1 ? "disk" : "disks")")
                Spacer(minLength: 0)
            }
            .font(.caption).foregroundStyle(.secondary).padding(12)
        }
    }

    private func sidebarRow(_ node: DeviceNode) -> some View {
        DeviceRow(node: node, isSelected: store.selection == node.id, showsDiskAssociation: !search.isEmpty)
            .tag(node.id)
            .contextMenu { DeviceContextMenu(node: node, store: store) }
    }
}

struct DeviceRow: View {
    let node: DeviceNode
    let isSelected: Bool
    var showsDiskAssociation = false

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: node.deviceSymbol)
                .font(.body).frame(width: 20)
                .foregroundColor(isSelected ? nil : .secondary)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 3) {
                Text(node.displayName).fontWeight(node.kind == .disk ? .medium : .regular)
                    .lineLimit(1).truncationMode(.tail)
                ViewThatFits(in: .horizontal) {
                    Text("\(node.kind == .disk ? node.disk.connectionType ?? node.kind.rawValue : node.kind.rawValue) · \(node.capacityDescription)")
                        .fixedSize(horizontal: true, vertical: false)
                    Text(node.capacityDescription).lineLimit(1).truncationMode(.tail)
                }
                .font(.caption).foregroundColor(isSelected ? nil : .secondary)
                if showsDiskAssociation && node.kind != .disk {
                    Text(node.disk.device.name).font(.caption).lineLimit(1)
                        .foregroundColor(isSelected ? nil : .secondary)
                }
            }
            .frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, node.kind == .disk ? 6 : 3)
        .help("\(node.displayName)\n\(node.device.identifier) · \(node.capacityDescription)\n\(node.disk.model ?? node.device.mediaName ?? node.connectionDescription)")
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(node.displayName), \(node.kind.rawValue), \(node.device.identifier), \(node.capacityDescription)")
    }
}
