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
    }
}

#Preview { ContentView() }
