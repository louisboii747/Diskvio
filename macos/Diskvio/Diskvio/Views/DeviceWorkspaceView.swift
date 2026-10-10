import SwiftUI

struct DeviceWorkspaceView: View {
    @ObservedObject var store: DiskStore
    @State private var showsInspector = true
    @State private var showsInspectorPopover = false

    var body: some View {
        GeometryReader { geometry in
            let allowsInlineInspector = WorkspaceLayout.allowsInlineInspector(width: geometry.size.width)
            HSplitView {
                detailContent
                    .frame(minWidth: WorkspaceLayout.contentMinimum, maxWidth: .infinity, maxHeight: .infinity)
                if showsInspector && allowsInlineInspector {
                    inspectorContent
                        .frame(minWidth: WorkspaceLayout.inspectorMinimum,
                               idealWidth: WorkspaceLayout.inspectorIdeal,
                               maxWidth: WorkspaceLayout.inspectorMaximum, maxHeight: .infinity)
                }
            }
            .toolbar {
                ToolbarItemGroup {
                    if let node = store.selectedNode, node.hasManagementActions {
                        Menu {
                            DeviceActionButtons(node: node, store: store)
                        } label: {
                            Label("Actions", systemImage: "ellipsis.circle")
                        }
                        .help("Actions for the selected device")
                    }
                    Button { Task { await store.refresh() } } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .disabled(store.isBusy)
                    .help("Refresh device information")
                    Button {
                        if allowsInlineInspector { showsInspector.toggle() }
                        else { showsInspectorPopover.toggle() }
                    } label: {
                        Label("Inspector", systemImage: "sidebar.right")
                    }
                    .keyboardShortcut("i", modifiers: [.command, .option])
                    .help(allowsInlineInspector ? "Show or hide device properties" : "Show device properties")
                    .popover(isPresented: $showsInspectorPopover, arrowEdge: .top) {
                        inspectorContent
                            .frame(width: WorkspaceLayout.inspectorMaximum, height: min(geometry.size.height, 600))
                    }
                }
            }
            .onChange(of: allowsInlineInspector) { _, inline in
                if inline { showsInspectorPopover = false }
            }
        }
        .navigationTitle(store.selectedNode?.device.name ?? "Diskvio")
    }

    private var inspectorContent: some View {
        Group {
            if let node = store.selectedNode {
                DeviceInspector(node: node)
            } else {
                Text("Select a device to inspect its properties.")
                    .foregroundStyle(.secondary).padding()
            }
        }
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
