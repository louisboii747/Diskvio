import AppKit
import SwiftUI

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
                store.request(action, from: node)
            }
            .disabled(store.isBusy)
            .help("\(action.title) \(node.device.name)")
        }
        if node.kind != .disk, node.disk.device.actions?.contains(.eject) == true {
            Button("Eject Disk", systemImage: "eject") {
                store.request(.eject, from: node)
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
