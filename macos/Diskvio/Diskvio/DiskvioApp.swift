import AppKit
import SwiftUI

@main
struct DiskvioApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .defaultSize(width: 1280, height: 780)
        .commands {
            DeviceMenuCommands()
            PartitionMenuCommands()
            USBCommands()
        }
    }
}

private struct DeviceMenuCommands: Commands {
    @FocusedObject private var store: DiskStore?

    var body: some Commands {
        CommandMenu("Device") {
            Button("Show in Finder", systemImage: "folder") {
                guard let mountPoint = selectedMountPoint else { return }
                NSWorkspace.shared.open(URL(fileURLWithPath: mountPoint, isDirectory: true))
            }
            .keyboardShortcut("o", modifiers: [.command, .option])
            .disabled(selectedMountPoint == nil)

            Button("Copy Device Identifier", systemImage: "doc.on.doc") {
                guard let identifier = store?.selectedNode?.device.identifier else { return }
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(identifier, forType: .string)
            }
            .keyboardShortcut("c", modifiers: [.command, .option])
            .disabled(store?.selectedNode == nil)

            Divider()

            Button("Refresh Devices", systemImage: "arrow.clockwise") {
                guard let store else { return }
                Task { await store.refresh() }
            }
            .keyboardShortcut("r", modifiers: .command)
            .disabled(store?.isBusy != false)
        }
    }

    private var selectedMountPoint: String? {
        guard let node = store?.selectedNode else { return nil }
        return node.device.mountPoint ?? node.volume?.mountedSnapshots?.first?.mountPoint
    }
}

private struct PartitionMenuCommands: Commands {
    @FocusedObject private var store: DiskStore?

    var body: some Commands {
        CommandMenu("Partition") {
            Button("Mount Volume", systemImage: DiskAction.mount.symbol) {
                request(.mount)
            }
            .disabled(!canRequest(.mount))

            Button("Unmount Volume", systemImage: DiskAction.unmount.symbol) {
                request(.unmount)
            }
            .disabled(!canRequest(.unmount))
        }
    }

    private func canRequest(_ action: DiskAction) -> Bool {
        guard let store else { return false }
        return store.canRequest(action, from: store.selectedNode)
    }

    private func request(_ action: DiskAction) {
        guard let store, let node = store.selectedNode else { return }
        store.request(action, from: node)
    }
}

private struct USBCommands: Commands {
    @FocusedObject private var store: DiskStore?

    var body: some Commands {
        CommandMenu("USB") {
            Button("Mount Volume", systemImage: DiskAction.mount.symbol) {
                request(.mount)
            }
            .disabled(!canRequest(.mount))

            Button("Unmount Volume", systemImage: DiskAction.unmount.symbol) {
                request(.unmount)
            }
            .disabled(!canRequest(.unmount))

            Divider()

            Button("Eject Disk", systemImage: DiskAction.eject.symbol) {
                request(.eject)
            }
            .keyboardShortcut("e", modifiers: [.command, .shift])
            .disabled(!canRequest(.eject))
        }
    }

    private func canRequest(_ action: DiskAction) -> Bool {
        guard let store else { return false }
        return store.canRequest(action, from: store.selectedUSBNode)
    }

    private func request(_ action: DiskAction) {
        guard let store, let node = store.selectedUSBNode else { return }
        store.request(action, from: node)
    }
}
