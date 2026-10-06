import SwiftUI

struct ContentView: View {
    @State private var disks: [Disk] = []
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Diskvio")
                    .font(.largeTitle)
                Spacer()
                Button("Refresh") {
                    Task { await refresh() }
                }
                .disabled(isLoading)
            }

            if isLoading {
                ProgressView("Discovering disks…")
            }

            if let errorMessage {
                Text(errorMessage)
                    .foregroundStyle(.red)
            }

            List(disks) { disk in
                VStack(alignment: .leading, spacing: 4) {
                    Text(disk.name)
                        .font(.headline)
                    Text("Disk \(disk.number) · \(ByteCountFormatter.string(fromByteCount: Int64(clamping: disk.sizeBytes), countStyle: .decimal)) · \(disk.busType)")
                        .foregroundStyle(.secondary)
                    if disk.partitionStyle != "Unknown" {
                        Text("Partition scheme: \(disk.partitionStyle)")
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 4)
            }
        }
        .padding()
        .frame(minWidth: 480, minHeight: 320)
        .task { await refresh() }
    }

    @MainActor
    private func refresh() async {
        isLoading = true
        errorMessage = nil
        do {
            disks = try await Task.detached(priority: .userInitiated) {
                try DiskService.listDisks()
            }.value
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }
}

#Preview {
    ContentView()
}
