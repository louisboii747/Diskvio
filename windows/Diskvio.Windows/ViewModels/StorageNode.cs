using Diskvio.Windows.Models;
namespace Diskvio.Windows.ViewModels;

public sealed record StorageNode(StorageDevice Device, string Kind, StorageDisk? Disk = null, StoragePartition? Partition = null, StorageVolume? Volume = null, AllocatableRegion? Region = null)
{
    public string Name => Kind == "Partition" && Partition is { } p ? PartitionPresentation.Name(p)
        : Kind == "Volume" ? PartitionPresentation.VolumeName(Volume, Device) : string.IsNullOrWhiteSpace(Device.Name) ? $"Disk {Disk?.Number}" : Device.Name;
    public string Capacity => Device.SizeBytes is { } bytes ? Models.Disk.FormatCapacity(bytes) : "Not reported";
    public ulong? UsedBytes => Device.UsedBytes ?? (Partition?.Volumes.Count == 1 ? Partition.Volumes[0].Device.UsedBytes : null);
    public ulong? AvailableBytes => Device.AvailableBytes ?? (Partition?.Volumes.Count == 1 ? Partition.Volumes[0].Device.AvailableBytes : null);
    public string Used => UsedBytes is { } bytes ? Models.Disk.FormatCapacity(bytes) : "Not reported";
    public string Available => AvailableBytes is { } bytes ? Models.Disk.FormatCapacity(bytes) : "Not reported";
    public string ExactCapacity => Device.SizeBytes is { } bytes ? $"{bytes:N0} bytes" : "Capacity not reported";
    public string GroupName => Disk?.Internal == true ? "Internal" : Disk?.Removable == true ? "Removable" : Disk?.Internal == false ? "External" : "Other devices";
    public string ReportedStatus => Device.Safety.Offline is { } offline ? offline ? "Offline" : "Online" : !string.IsNullOrEmpty(Status) ? Status : !string.IsNullOrEmpty(Health) ? Health : "Status not reported";
    public string Summary => Kind == "Physical disk" ? $"Disk {Disk?.Number} · {Capacity} · {Disk?.ConnectionType ?? "Connection not reported"}" : $"{Kind} · {Capacity} · {Filesystem}";
    public string BrowserSummary => Kind == "Physical disk" ? Summary : $"{Kind} · {Capacity}\n{Disk?.Device.Name ?? "Disk association unavailable"}";
    public string Filesystem => Region is not null ? "Not applicable" : Kind == "Partition" && Partition is { } p ? Empty(PartitionPresentation.Filesystems(p), "Not reported") : Device.Filesystem?.Name ?? "Not reported";
    public string Location => Region is not null ? "No volume" : Kind == "Partition" && Partition is { } p ? Empty(string.Join(", ", p.Volumes.Select(v => v.Device.DriveLetter is { } l ? $"{l}:" : v.Device.MountPoint).Where(s => !string.IsNullOrWhiteSpace(s))), "—") : Device.DriveLetter is { } letter ? $"{letter}:" : Device.MountPoint ?? "—";
    public double Indent => Kind == "Volume" && Partition is not null ? 20 : 0;
    public string Type => Region is { } region ? region.Validation.Valid ? "Unallocated · preview eligible" : "Unallocated · planning unavailable" : Kind == "Partition" && Partition is { } p ? PartitionPresentation.RoleName(p) ?? p.ContentType ?? "Not reported" : Kind;
    public string Health => string.Join(", ", (Kind == "Partition" && Partition is { } p ? p.Volumes.Select(v => v.Device.HealthStatus).Append(Device.HealthStatus) : [Device.HealthStatus]).Where(s => !string.IsNullOrWhiteSpace(s)).Distinct());
    public string Status => string.Join(", ", (Kind == "Partition" && Partition is { } p ? p.Volumes.SelectMany(v => v.Device.OperationalStatus).Concat(Device.OperationalStatus) : Device.OperationalStatus).Distinct());
    public string Badges => string.Join(" · ", Tags());
    private IEnumerable<string> Tags()
    {
        var s = Device.Safety;
        if (s.System == true || Kind == "Partition" && Partition is { } p && PartitionPresentation.Role(p) == "efi_system") yield return "System";
        if (s.Boot == true) yield return "Boot";
        if (s.Recovery == true) yield return "Recovery";
        if (s.PageFile == true) yield return "Paging file";
        if (s.ReadOnly == true || Disk?.Device.Safety.ReadOnly == true) yield return "Read-only";
        if (s.Offline == true || Disk?.Device.Safety.Offline == true) yield return "Offline";
        if (s.Hidden == true) yield return "Hidden";
        if (Disk?.Internal == false) yield return "External";
        if (Disk?.Removable == true) yield return "Removable";
        if (Disk?.ConnectionType?.Equals("USB", StringComparison.OrdinalIgnoreCase) == true) yield return "USB";
    }
    private static string Empty(string value, string fallback) => string.IsNullOrWhiteSpace(value) ? fallback : value;
    public override string ToString() => AccessibleName;
    public string AccessibleName => $"{Name}, {Summary}, {Location}, {Badges}, {Device.Identifier}, {ExactCapacity}, used {Used}, available {Available}";
    public string Icon => Kind == "Volume" ? "\uE8B7" : Disk?.ConnectionType?.Equals("USB", StringComparison.OrdinalIgnoreCase) == true ? "\uE88E" : "\uEDA2";
    public string MountLocation => Device.MountPoints.Count > 0 ? string.Join(", ", Device.MountPoints) : Device.MountPoint ?? "Not mounted";
    public IReadOnlyList<StorageNode> Children => Kind == "Physical disk" && Disk is not null
        ? Disk.Partitions.Select(p => new StorageNode(p.Device, "Partition", Disk, p)).ToList()
        : Kind == "Partition" && Partition is not null
            ? Partition.Volumes.Select(v => new StorageNode(v.Device, "Volume", Disk, Partition, v)).ToList() : [];
}
public sealed record InspectorProperty(string Label, string Value);
