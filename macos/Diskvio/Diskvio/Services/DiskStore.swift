import Combine
import Foundation

@MainActor
final class DiskStore: ObservableObject {
    @Published private(set) var inventory = DiskInventory()
    @Published var selection: String?
    @Published private(set) var isLoading = false
    @Published private(set) var errorMessage: String?
    @Published private(set) var lastUpdated: Date?
    @Published private(set) var operationInProgress: String?
    @Published private(set) var operationMessage: String?
    @Published private(set) var operationFailed = false
    @Published private(set) var monitoringError: String?
    @Published var pendingOperation: PendingDiskOperation?

    private let loadInventory: @Sendable () throws -> DiskInventory
    private let runOperation: @Sendable (OperationRequest) throws -> OperationOutcome
    private let loadCapabilities: @Sendable (String) throws -> OperationCapabilities
    private let validateOperation: @Sendable (OperationRequest) throws -> OperationValidation

    init(loadInventory: @escaping @Sendable () throws -> DiskInventory = { try DiskService.inventory() },
         runOperation: @escaping @Sendable (OperationRequest) throws -> OperationOutcome = { try DiskService.perform($0) },
         loadCapabilities: @escaping @Sendable (String) throws -> OperationCapabilities = { try DiskService.supportedOperations(identifier: $0) },
         validateOperation: @escaping @Sendable (OperationRequest) throws -> OperationValidation = { try DiskService.validate($0) }) {
        self.loadInventory = loadInventory
        self.runOperation = runOperation
        self.loadCapabilities = loadCapabilities
        self.validateOperation = validateOperation
    }

    private var monitor: DiskMonitor?
    private var eventTask: Task<Void, Never>?
    private var refreshTask: Task<Void, Never>?
    private var needsRefresh = false
    private var isActive = false
    private var generation: UInt64 = 0

    var nodes: [DeviceNode] { inventory.nodes }
    var selectedNode: DeviceNode? { nodes.flatMap(\.flattened).first { $0.id == selection } }
    var isBusy: Bool { isLoading || operationInProgress != nil }
    var selectedUSBNode: DeviceNode? {
        guard let node = selectedNode,
              node.disk.internal == false,
              node.disk.connectionType?.localizedCaseInsensitiveContains("USB") == true else { return nil }
        return node
    }

    func target(for action: DiskAction, from node: DeviceNode) -> DeviceNode? {
        if node.device.actions?.contains(action) == true { return node }
        guard action == .eject,
              node.kind != .disk,
              node.disk.device.actions?.contains(.eject) == true else { return nil }
        return DeviceNode(
            id: node.id,
            kind: .disk,
            device: node.disk.device,
            disk: node.disk
        )
    }

    func canRequest(_ action: DiskAction, from node: DeviceNode?) -> Bool {
        guard !isBusy, let node else { return false }
        return target(for: action, from: node) != nil
    }

    func request(_ action: DiskAction, from node: DeviceNode) {
        guard let target = target(for: action, from: node), !isBusy else { return }
        if action == .mount {
            Task { await perform(action, on: target) }
        } else {
            pendingOperation = PendingDiskOperation(action: action, node: target)
        }
    }

    func confirmPendingOperation() {
        guard let pendingOperation else { return }
        self.pendingOperation = nil
        Task { await perform(pendingOperation.action, on: pendingOperation.node) }
    }

    func start() async {
        guard monitor == nil else { return }
        isActive = true
        await refresh()
        guard isActive, !Task.isCancelled else { return }
        do {
            let monitor = try DiskMonitor()
            self.monitor = monitor
            monitoringError = nil
            eventTask = Task { [weak self, events = monitor.events] in
                for await event in events {
                    guard !Task.isCancelled else { break }
                    self?.receive(event)
                }
            }
        } catch {
            monitoringError = error.localizedDescription
        }
    }

    func stopMonitoring() {
        isActive = false
        refreshTask?.cancel()
        refreshTask = nil
        eventTask?.cancel()
        eventTask = nil
        monitor?.stop()
        monitor = nil
    }

    func refresh() async {
        guard !isBusy else { needsRefresh = true; return }
        refreshTask?.cancel()
        refreshTask = nil
        needsRefresh = false
        let scanGeneration = generation
        isLoading = true
        do {
            let loader = loadInventory
            let updated = try await Task.detached(priority: .userInitiated) { try loader() }.value
            // A removal during discovery invalidates the snapshot. The immediate
            // removal stays visible; one coalesced follow-up retrieves current data.
            if scanGeneration == generation {
                inventory = updated
                reconcileSelection()
                errorMessage = nil
                lastUpdated = Date()
            } else {
                needsRefresh = true
            }
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
        if needsRefresh { scheduleRefresh() }
    }

    func perform(_ action: DiskAction, on node: DeviceNode) async {
        guard !isBusy, node.device.actions?.contains(action) == true,
              let identity = node.device.identityToken else { return }
        let request = OperationRequest(action: action, identifier: node.device.identifier, expectedIdentity: identity)
        operationInProgress = "\(action.title) · \(node.device.name)"
        operationMessage = nil
        do {
            let capabilitiesLoader = loadCapabilities
            let capabilities = try await Task.detached(priority: .userInitiated) {
                try capabilitiesLoader(request.identifier)
            }.value
            guard capabilities.actions.contains(action) else {
                throw DiskServiceError.operationFailed(BackendOperationError(
                    code: "unsupported_operation",
                    message: "\(action.title) is no longer available for this device.",
                    platformCode: nil
                ))
            }
            let validator = validateOperation
            _ = try await Task.detached(priority: .userInitiated) {
                try validator(request)
            }.value
            let operation = runOperation
            let outcome = try await Task.detached(priority: .userInitiated) { try operation(request) }.value
            operationMessage = outcome.message
            operationFailed = false
        } catch {
            operationMessage = "\(action.title) failed: \(error.localizedDescription)"
            operationFailed = true
        }
        operationInProgress = nil
        await refresh()
    }

    private func reconcileSelection() {
        let flattened = nodes.flatMap(\.flattened)
        if selection == nil || !flattened.contains(where: { $0.id == selection }) {
            selection = nodes.first?.id
        }
    }

    func receive(_ event: DiskEvent) {
        let knownIDs = Set(inventory.disks.flatMap { [$0.device.identifier] + $0.partitions.map { $0.device.identifier } }
            + inventory.apfsContainers.flatMap { [$0.device.identifier] + $0.volumes.map { $0.device.identifier } })
        let relevant: Bool
        switch event.kind {
        case .appeared:
            // Initial enumeration of already-known devices causes no extra scan.
            relevant = !knownIDs.contains(event.identifier) && (event.physicalCandidate || event.wholeIdentifier.map(knownIDs.contains) == true)
        case .disappeared, .changed:
            relevant = knownIDs.contains(event.identifier)
        }
        guard relevant else { return }
        generation &+= 1
        if event.kind == .disappeared {
            let removedStoreIDs = Set(inventory.disks.filter { $0.device.identifier == event.identifier }
                .flatMap { [$0.device.identifier] + $0.partitions.map { $0.device.identifier } })
            inventory.disks.removeAll { $0.device.identifier == event.identifier }
            for index in inventory.disks.indices {
                inventory.disks[index].removePartition(identifier: event.identifier)
            }
            inventory.apfsContainers.removeAll {
                $0.device.identifier == event.identifier || !$0.physicalStoreIDs.allSatisfy { !removedStoreIDs.contains($0) && $0 != event.identifier }
            }
            for index in inventory.apfsContainers.indices {
                inventory.apfsContainers[index].removeVolume(identifier: event.identifier)
            }
            reconcileSelection()
        }
        if isBusy { needsRefresh = true } else { scheduleRefresh() }
    }

    private func scheduleRefresh() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            do { try await Task.sleep(for: .milliseconds(450)) } catch { return }
            guard !Task.isCancelled else { return }
            await self?.refresh()
        }
    }

    isolated deinit { stopMonitoring() }
}
