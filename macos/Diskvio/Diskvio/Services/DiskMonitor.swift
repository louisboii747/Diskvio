import DiskArbitration
import Foundation

nonisolated struct DiskEvent: Sendable {
    enum Kind: Sendable { case appeared, disappeared, changed }
    let kind: Kind
    let identifier: String
    let wholeIdentifier: String?
    let physicalCandidate: Bool
}

/// The C context owns only a Sendable stream continuation, never UI state.
private nonisolated final class DiskEventSink: Sendable {
    let continuation: AsyncStream<DiskEvent>.Continuation
    init(_ continuation: AsyncStream<DiskEvent>.Continuation) { self.continuation = continuation }

    func emit(_ disk: DADisk, kind: DiskEvent.Kind) {
        guard let name = DADiskGetBSDName(disk) else { return }
        let description = DADiskCopyDescription(disk) as? [String: Any] ?? [:]
        let whole = DADiskCopyWholeDisk(disk).flatMap { DADiskGetBSDName($0).map { String(cString: $0) } }
        let isWhole = description[kDADiskDescriptionMediaWholeKey as String] as? Bool == true
        let protocolName = description[kDADiskDescriptionDeviceProtocolKey as String] as? String
        let isVirtual = ["Disk Image", "Virtual Interface", "APFS"].contains(protocolName ?? "")
        continuation.yield(DiskEvent(kind: kind, identifier: String(cString: name), wholeIdentifier: whole, physicalCandidate: isWhole && !isVirtual))
    }
}

private nonisolated let appearedCallback: DADiskAppearedCallback = { disk, context in
    guard let context else { return }
    Unmanaged<DiskEventSink>.fromOpaque(context).takeUnretainedValue().emit(disk, kind: .appeared)
}
private nonisolated let disappearedCallback: DADiskDisappearedCallback = { disk, context in
    guard let context else { return }
    Unmanaged<DiskEventSink>.fromOpaque(context).takeUnretainedValue().emit(disk, kind: .disappeared)
}
private nonisolated let changedCallback: DADiskDescriptionChangedCallback = { disk, _, context in
    guard let context else { return }
    Unmanaged<DiskEventSink>.fromOpaque(context).takeUnretainedValue().emit(disk, kind: .changed)
}

@MainActor
final class DiskMonitor {
    let events: AsyncStream<DiskEvent>
    private var session: DASession?
    private var context: UnsafeMutableRawPointer?

    init() throws {
        guard let session = DASessionCreate(kCFAllocatorDefault) else {
            throw DiskServiceError.discoveryFailed("Could not create a Disk Arbitration session. Check macOS permissions and choose Refresh.")
        }
        let stream = AsyncStream<DiskEvent>.makeStream(bufferingPolicy: .unbounded)
        events = stream.stream
        self.session = session
        context = Unmanaged.passRetained(DiskEventSink(stream.continuation)).toOpaque()
        DARegisterDiskAppearedCallback(session, nil, appearedCallback, context)
        DARegisterDiskDisappearedCallback(session, nil, disappearedCallback, context)
        // Capacity usage changes do not trigger scans. Watch topology/name/mount
        // metadata only; arrival/removal callbacks handle new APFS volumes.
        let keys = [kDADiskDescriptionVolumePathKey, kDADiskDescriptionVolumeNameKey,
                    kDADiskDescriptionVolumeUUIDKey, kDADiskDescriptionMediaSizeKey,
                    kDADiskDescriptionMediaContentKey, kDADiskDescriptionDeviceInternalKey]
        DARegisterDiskDescriptionChangedCallback(session, nil, keys as CFArray, changedCallback, context)
        DASessionSetDispatchQueue(session, .main)
    }

    func stop() {
        guard let session else { return }
        // Callbacks and cleanup both run on the main queue. Unscheduling and
        // releasing the session removes its registrations before freeing context.
        DASessionSetDispatchQueue(session, nil)
        // Disk Arbitration's C API requires a raw callback address to unregister.
        DAUnregisterCallback(session, unsafeBitCast(appearedCallback, to: UnsafeMutableRawPointer.self), context)
        DAUnregisterCallback(session, unsafeBitCast(disappearedCallback, to: UnsafeMutableRawPointer.self), context)
        DAUnregisterCallback(session, unsafeBitCast(changedCallback, to: UnsafeMutableRawPointer.self), context)
        self.session = nil
        if let context {
            let sink = Unmanaged<DiskEventSink>.fromOpaque(context).takeRetainedValue()
            sink.continuation.finish()
            self.context = nil
        }
    }

    isolated deinit { stop() }
}
