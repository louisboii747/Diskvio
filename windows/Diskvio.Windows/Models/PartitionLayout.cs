namespace Diskvio.Windows.Models;

public sealed record PartitionSegment(StoragePartition? Partition, ulong Offset, ulong Length)
{
    public string Label => Partition is { } p ? PartitionPresentation.Name(p) : "Unmapped space / metadata";
    public string Category => Partition is { } p ? p.Device.Safety.Recovery == true ? "recovery" : p.Device.Safety.System == true || p.Device.Safety.Boot == true ? "efi_system" : PartitionPresentation.Role(p) : "unmapped";
    public string Description => $"{Label} · {Disk.FormatCapacity(Length)} · offset {Offset:N0} bytes" +
        (Partition is { } p ? $" · {PartitionPresentation.RoleName(p) ?? "Type not reported"} · {PartitionPresentation.Filesystems(p)} · {p.Device.Identifier}" : " · Not an authorized allocation target");
}

public sealed record PartitionLayout(bool IsPhysical, string Explanation, IReadOnlyList<PartitionSegment> Segments)
{
    // Pin tiny regions to a usable width, then distribute the rest by capacity.
    public IReadOnlyList<double> VisualWidths(double availableWidth)
    {
        if (Segments.Count == 0) return [];
        var minima = Segments.Select(s => s.Partition is null ? 4d : 36d).ToArray();
        var remaining = Math.Max(availableWidth, minima.Sum());
        var widths = new double[Segments.Count];
        var pending = Enumerable.Range(0, Segments.Count).ToList();
        while (pending.Count > 0)
        {
            var total = pending.Sum(i => (double)Segments[i].Length);
            var tiny = pending.Where(i => total <= 0 || remaining * Segments[i].Length / total < minima[i]).ToList();
            if (tiny.Count == 0) { foreach (var i in pending) widths[i] = remaining * Segments[i].Length / total; break; }
            foreach (var i in tiny) { widths[i] = minima[i]; remaining -= minima[i]; pending.Remove(i); }
        }
        return widths;
    }
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
        return new(true, "Capacity proportions; tiny partitions have a minimum display width. Dark gaps are unmapped space or metadata, not verified free regions. Scroll dense layouts to inspect every partition.", result);
    }
    private static PartitionLayout Unknown(List<StoragePartition> partitions) => new(false,
        "Physical positioning is unavailable. Reported capacities are shown with minimum widths for tiny partitions; no gaps are inferred.",
        partitions.Where(p => p.Device.SizeBytes is > 0).Select(p => new PartitionSegment(p, 0, p.Device.SizeBytes!.Value)).ToList());
}
