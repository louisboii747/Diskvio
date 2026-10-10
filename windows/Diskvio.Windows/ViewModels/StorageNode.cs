using Diskvio.Windows.Models;
namespace Diskvio.Windows.ViewModels;

public sealed record StorageNode(StorageDevice Device, string Kind, StorageDisk? Disk = null, StoragePartition? Partition = null)
{
    public string Name => string.IsNullOrWhiteSpace(Device.Name) ? Device.Identifier : Device.Name;
    public string Capacity => Device.SizeBytes is { } bytes ? Models.Disk.FormatCapacity(bytes) : "Not reported";
    public string Summary => Kind == "Physical disk" ? $"Disk {Disk?.Number} · {Capacity} · {Disk?.ConnectionType ?? "Not reported"}" : $"{Kind} · {Capacity}";
    public override string ToString() => AccessibleName;
    public string AccessibleName => $"{Name}, {Summary}, {Device.Identifier}";
    public string Icon => Kind == "Volume" ? "\uE8B7" : Disk?.ConnectionType?.Equals("USB", StringComparison.OrdinalIgnoreCase) == true ? "\uE88E" : "\uEDA2";
    public string MountLocation => Device.MountPoints.Count > 0 ? string.Join(", ", Device.MountPoints) : Device.MountPoint ?? "Not mounted";
    public IReadOnlyList<StorageNode> Children => Kind == "Physical disk" && Disk is not null
        ? Disk.Partitions.Select(p => new StorageNode(p.Device, "Partition", Disk, p)).ToList()
        : Kind == "Partition" && Partition is not null
            ? Partition.Volumes.Select(v => new StorageNode(v.Device, "Volume", Disk, Partition)).ToList() : [];
}
public sealed record InspectorProperty(string Label, string Value);
