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

    private let loadInventory: @Sendable () throws -> DiskInventory
    private let runOperation: @Sendable (OperationRequest) throws -> OperationOutcome

    init(loadInventory: @escaping @Sendable () throws -> DiskInventory = { try DiskService.inventory() },
         runOperation: @escaping @Sendable (OperationRequest) throws -> OperationOutcome = { try DiskService.perform($0) }) {
        self.loadInventory = loadInventory
        self.runOperation = runOperation
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
