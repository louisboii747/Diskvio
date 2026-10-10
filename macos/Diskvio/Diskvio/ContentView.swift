import SwiftUI

struct ContentView: View {
    @StateObject private var store = DiskStore()

    var body: some View {
        NavigationSplitView {
            DeviceSidebar(store: store)
                .navigationSplitViewColumnWidth(min: WorkspaceLayout.sidebarMinimum,
                                                ideal: WorkspaceLayout.sidebarIdeal,
                                                max: WorkspaceLayout.sidebarMaximum)
        } detail: {
            DeviceWorkspaceView(store: store)
        }
        .navigationSplitViewStyle(.balanced)
        .frame(minWidth: WorkspaceLayout.windowMinimumWidth, minHeight: 600)
        .task { await store.start() }
        .onDisappear { store.stopMonitoring() }
        .focusedSceneObject(store)
        .confirmationDialog(
            store.pendingOperation?.title ?? "Confirm Disk Operation",
            isPresented: Binding(
                get: { store.pendingOperation != nil },
                set: { if !$0 { store.pendingOperation = nil } }
            ),
            presenting: store.pendingOperation
        ) { operation in
            Button(operation.confirmationTitle, role: operation.action == .eject ? .destructive : nil) {
                store.confirmPendingOperation()
            }
            Button("Cancel", role: .cancel) {
                store.pendingOperation = nil
            }
        } message: { operation in
            Text(operation.message)
        }
    }
}

#Preview { ContentView() }
