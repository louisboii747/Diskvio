import AppKit
import SwiftUI

struct ContentView: View {
    @StateObject private var store = DiskStore()
    @State private var showsInspector = true

    var body: some View {
        NavigationSplitView {
            List(selection: $store.selection) {
                Section("Storage Devices") {
                    ForEach(store.nodes) { node in
                        SidebarBranch(node: node, store: store)
                    }
                }
            }
            .listStyle(.sidebar)
            .navigationSplitViewColumnWidth(min: 220, ideal: 275, max: 400)
            .overlay {
                if store.nodes.isEmpty && store.isLoading {
                    ProgressView("Discovering disks…")
                }
            }
            .safeAreaInset(edge: .bottom) {
                HStack(spacing: 6) {
                    if store.isLoading { ProgressView().controlSize(.small) }
                    Text(store.isLoading ? "Refreshing…" : "\(store.inventory.disks.count) physical disks")
                    Spacer()
                }
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(12)
            }
        } detail: {
            HSplitView {
                detailContent.frame(minWidth: 350, maxWidth: .infinity, maxHeight: .infinity)
                if showsInspector {
                    Group {
                        if let node = store.selectedNode {
                            DeviceInspector(node: node)
                        } else {
                            Text("Select a device to inspect its properties.")
                                .foregroundStyle(.secondary).padding()
                        }
                    }
                    .frame(minWidth: 235, idealWidth: 275, maxWidth: 360, maxHeight: .infinity)
                }
            }
            .navigationTitle(store.selectedNode?.device.name ?? "Diskvio")
            .toolbar {
                ToolbarItemGroup {
                    if let node = store.selectedNode {
                        DeviceActionButtons(node: node, store: store)
                    }
                    Button { Task { await store.refresh() } } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(store.isBusy)
                    .help("Refresh device information")
                    Button { showsInspector.toggle() } label: {
                        Label("Inspector", systemImage: "sidebar.right")
                    }
                    .help("Show or hide device properties")
                }
            }
        }
        .frame(minWidth: 880, minHeight: 520)
        .task { await store.start() }
        .onDisappear { store.stopMonitoring() }
    }
    private var detailContent: some View {
        VStack(spacing: 0) {
            if let progress = store.operationInProgress {
                HStack {
                    ProgressView().controlSize(.small)
                    Text(progress)
                    Spacer()
                }.padding(12)
                Divider()
            }
            if let message = store.operationMessage {
                NoticeView(title: store.operationFailed ? "Operation failed" : "Operation complete", message: message,
                           symbol: store.operationFailed ? "exclamationmark.triangle" : "checkmark.circle")
            }
            if let message = store.monitoringError {
                NoticeView(title: "Live detection unavailable", message: message, symbol: "externaldrive.badge.exclamationmark")
            }
            if let error = store.errorMessage, !store.nodes.isEmpty {
                NoticeView(title: "Refresh failed", message: error, symbol: "exclamationmark.triangle")
            }
            if !store.inventory.warnings.isEmpty {
                DisclosureGroup("Some device information is unavailable") {
                    ForEach(store.inventory.warnings, id: \.self) { warning in
                        Text(warning).font(.caption).textSelection(.enabled)
                    }
                }
                .padding(12)
                Divider()
            }
            if let node = store.selectedNode {
                DeviceDetailView(node: node, store: store)
            } else if store.isLoading {
                ProgressView("Discovering disks…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error = store.errorMessage {
                ContentUnavailableView {
                    Label("Disk Discovery Failed", systemImage: "externaldrive.badge.exclamationmark")
                } description: {
                    Text(error)
                } actions: {
                    Button("Try Again") { Task { await store.refresh() } }
                }
            } else {
                ContentUnavailableView("No Physical Disks Found", systemImage: "externaldrive", description: Text("Connect a storage device or choose Refresh to scan again."))
            }
        }
    }

}

private struct SidebarBranch: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore
    @State private var expanded = true

    var body: some View {
        if let children = node.children {
            DisclosureGroup(isExpanded: $expanded) {
                ForEach(children) { child in
                    SidebarBranch(node: child, store: store)
                }
            } label: {
                DeviceRow(node: node)
            }
            .tag(node.id)
            .contextMenu { DeviceContextMenu(node: node, store: store) }
        } else {
            DeviceRow(node: node)
                .tag(node.id)
                .contextMenu { DeviceContextMenu(node: node, store: store) }
        }
    }
}

struct DeviceRow: View {
    let node: DeviceNode
    var body: some View {
        Label {
            VStack(alignment: .leading, spacing: 2) {
                Text(node.device.name).lineLimit(1)
                Text("\(node.kind.rawValue) · \(node.capacityDescription)")
                    .font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
        } icon: {
            Image(systemName: node.kind.symbol)
        }
        .padding(.vertical, 3)
        .help("\(node.device.name) — \(node.device.identifier)")
    }
}

struct DeviceContextMenu: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore

    var body: some View {
        DeviceActionButtons(node: node, store: store)
        if let mountPoint = node.device.mountPoint ?? node.volume?.mountedSnapshots?.first?.mountPoint {
            Button("Show in Finder", systemImage: "folder") {
                NSWorkspace.shared.open(URL(fileURLWithPath: mountPoint, isDirectory: true))
            }
        }
        Button("Copy Device Identifier", systemImage: "doc.on.doc") {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(node.device.identifier, forType: .string)
        }
        Divider()
        Button("Refresh", systemImage: "arrow.clockwise") { Task { await store.refresh() } }
            .disabled(store.isBusy)
    }
}

struct DeviceActionButtons: View {
    let node: DeviceNode
    @ObservedObject var store: DiskStore

    var body: some View {
        ForEach(node.device.actions ?? [], id: \.rawValue) { action in
            Button(action.title, systemImage: action.symbol) {
                Task { await store.perform(action, on: node) }
            }
            .disabled(store.isBusy)
            .help("\(action.title) \(node.device.name)")
        }
        if node.kind != .disk, node.disk.device.actions?.contains(.eject) == true {
            Button("Eject Disk", systemImage: "eject") {
                let diskNode = DeviceNode(id: node.id, kind: .disk, device: node.disk.device, disk: node.disk)
                Task { await store.perform(.eject, on: diskNode) }
            }
            .disabled(store.isBusy)
            .help("Eject \(node.disk.device.name) and its volumes")
        }
    }
}

struct NoticeView: View {
    let title: String
    let message: String
    let symbol: String
    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: symbol).foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 3) {
                Text(title).fontWeight(.medium)
                Text(message).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary)
    }
}

#Preview { ContentView() }
