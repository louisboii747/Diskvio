namespace Diskvio.Windows.Models;

public sealed record PartitionSegment(StoragePartition? Partition, ulong Offset, ulong Length)
{
    public string Label => Partition?.Device.Name ?? "Unallocated / partition metadata";
    public string Description => $"{Label} · {Disk.FormatCapacity(Length)} · offset {Offset:N0} bytes";
}

public sealed record PartitionLayout(bool IsPhysical, string Explanation, IReadOnlyList<PartitionSegment> Segments)
{
    public static PartitionLayout Create(StorageDisk disk)
    {
        var partitions = disk.Partitions;
        // Require complete, non-overlapping geometry before representing any gaps.
        if (disk.Device.SizeBytes is not > 0 || partitions.Count == 0 ||
            partitions.Any(p => p.OffsetBytes is null || p.Device.SizeBytes is not > 0))
            return Unknown(partitions);
        var size = disk.Device.SizeBytes.Value;
        var ordered = partitions.OrderBy(p => p.OffsetBytes).ToList();
        ulong end = 0;
        var result = new List<PartitionSegment>();
        foreach (var p in ordered)
        {
            var offset = p.OffsetBytes!.Value;
            var length = p.Device.SizeBytes!.Value;
            if (offset < end || offset > size || length > size - offset) return Unknown(partitions);
            if (offset > end) result.Add(new(null, end, offset - end));
            result.Add(new(p, offset, length));
            end = offset + length;
        }
        if (end < size) result.Add(new(null, end, size - end));
        return new(true, "Physical scale. Neutral regions include unallocated space and partition-table metadata.", result);
    }
    private static PartitionLayout Unknown(List<StoragePartition> partitions) => new(false,
        "Physical positioning is unavailable. Known partition capacities are shown separately; no gaps are inferred.",
        partitions.Where(p => p.Device.SizeBytes is > 0).Select(p => new PartitionSegment(p, 0, p.Device.SizeBytes!.Value)).ToList());
}
