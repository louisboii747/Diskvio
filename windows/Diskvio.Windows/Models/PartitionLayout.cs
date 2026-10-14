namespace Diskvio.Windows.Models;

public sealed record PartitionSegment(StoragePartition? Partition, ulong Offset, ulong Length, AllocatableRegion? Region = null, string? Proposal = null)
{
    public string? SelectionId => Region is { } region ? PartitionPlanInput.RegionId(region) : Partition?.Device.Identifier;
    public string Label => Proposal ?? (Region is not null ? "Unallocated space" : Partition is { } p ? PartitionPresentation.Name(p) : "Unmapped space / metadata");
    public bool HasKnownSize => Partition is null || Partition.Device.SizeBytes is not null;
    public string DisplayCapacity => HasKnownSize ? Disk.FormatCapacity(Length) : "Capacity not reported";
    public string Category => Proposal is not null ? "proposal" : Region is not null ? "unallocated" : Partition is { } p ? p.Device.Safety.Recovery == true ? "recovery" : p.Device.Safety.System == true || p.Device.Safety.Boot == true ? "efi_system" : PartitionPresentation.Role(p) : "unmapped";
    public string KindLabel => Category switch
    {
        "proposal" => "Preview only", "unallocated" => "Unallocated", "efi_system" => "EFI / system",
        "recovery" => "Recovery", "microsoft_reserved" => "Reserved", "basic_data" => "Basic data",
        "unmapped" => "Metadata / unmapped", _ => "Unknown type"
    };
    public string Description => $"{Label} · {DisplayCapacity}" + (HasKnownSize ? $" ({Length:N0} bytes)" : "") +
        (Partition is not null && Partition.OffsetBytes is null ? " · offset not reported" : $" · offset {Offset:N0} bytes") +
        (Partition is { } p ? $" · {PartitionPresentation.RoleName(p) ?? "Type not reported"} · {PartitionPresentation.Filesystems(p)} · {string.Join(", ", p.Volumes.SelectMany(v => v.Device.MountPoints))} · {p.Device.Identifier}" : Region is { } region ? $" · Safely allocatable: {region.MaximumAllocatableBytes:N0} bytes · {PartitionPlanInput.Issues(region.Validation)}" : " · Not an authorized allocation target");
}

public sealed record PartitionLayout(bool IsPhysical, string Explanation, IReadOnlyList<PartitionSegment> Segments)
{
    public IReadOnlyList<double> VisualWidths(double availableWidth)
    {
        if (Segments.Count == 0) return [];
        var minima = Segments.Select(s => s.Region is not null || s.Proposal is not null ? 48d : s.Partition is not null ? 36d : 4d).ToArray();
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
    public static PartitionLayout Create(StorageDisk disk, DiskPlanningDiscovery? discovery = null)
    {
        if (discovery is { Geometry: { } geometry, LayoutSnapshot: not null } && discovery.Identifier == disk.Device.Identifier)
        {
            var authoritative = new List<PartitionSegment>();
            foreach (var deletion in discovery.Partitions)
            {
                var p = deletion.Partition;
                var stored = disk.Partitions.FirstOrDefault(item => string.Equals(item.Device.PartitionUuid?.Trim('{', '}'), p.PartitionId?.Trim('{', '}'), StringComparison.OrdinalIgnoreCase) && p.PartitionId is not null);
                if (p.OffsetBytes is not { } offset || p.SizeBytes is not { } length || stored is null || stored.OffsetBytes != offset || stored.Device.SizeBytes != length)
                    return Create(disk);
                authoritative.Add(new(stored, offset, length));
            }
            authoritative.AddRange(discovery.Regions.Select(r => new PartitionSegment(null, r.Identity.StartOffsetBytes, r.CapacityBytes, r)));
            ulong authoritativeEnd = 0;
            var verified = new List<PartitionSegment>();
            foreach (var segment in authoritative.OrderBy(s => s.Offset))
            {
                if (segment.Offset < authoritativeEnd || segment.Offset > geometry.DiskSizeBytes || segment.Length > geometry.DiskSizeBytes - segment.Offset) return Create(disk);
                if (segment.Offset > authoritativeEnd) verified.Add(new(null, authoritativeEnd, segment.Offset - authoritativeEnd));
                verified.Add(segment);
                authoritativeEnd = checked(segment.Offset + segment.Length);
            }
            if (authoritativeEnd < geometry.DiskSizeBytes) verified.Add(new(null, authoritativeEnd, geometry.DiskSizeBytes - authoritativeEnd));
            return new(true, "Unallocated regions come from the Rust backend. Metadata is separate. Small targets have minimum visual widths; exact byte values remain authoritative.", verified);
        }
        var partitions = disk.Partitions;
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

    public PartitionLayout? Preview(PartitionPlanningResult preview)
    {
        if (!preview.Validation.Valid || !preview.PreviewOnly || preview.ExecutionSupported) return null;
        var projected = new List<PartitionSegment>();
        var replaced = false;
        foreach (var segment in Segments)
        {
            if (preview.CreatePreview is { } create && segment.Region?.Identity == create.Region)
            {
                var end = checked(segment.Offset + segment.Length);
                if (create.StartOffsetBytes < segment.Offset || create.EndOffsetBytes > end || create.StartOffsetBytes >= create.EndOffsetBytes || create.EndOffsetBytes - create.StartOffsetBytes != create.AllocatedSizeBytes) return null;
                if (create.StartOffsetBytes > segment.Offset) projected.Add(new(null, segment.Offset, create.StartOffsetBytes - segment.Offset, Proposal: "Remaining unallocated (preview)"));
                projected.Add(new(null, create.StartOffsetBytes, create.AllocatedSizeBytes, Proposal: "Proposed basic data (preview)"));
                if (create.EndOffsetBytes < end) projected.Add(new(null, create.EndOffsetBytes, end - create.EndOffsetBytes, Proposal: "Remaining unallocated (preview)"));
                replaced = true;
            }
            else if (preview.DeletePreview is { Eligible: true, Validation.Valid: true } deletion && segment.Partition is { } part &&
                string.Equals(part.Device.PartitionUuid?.Trim('{', '}'), deletion.Partition.PartitionId?.Trim('{', '}'), StringComparison.OrdinalIgnoreCase) && deletion.Partition.PartitionId is not null)
            {
                projected.Add(new(null, segment.Offset, segment.Length, Proposal: "Unallocated after deletion (preview)"));
                replaced = true;
            }
            else projected.Add(segment);
        }
        return replaced ? new(IsPhysical, "Proposed layout only. No partition or filesystem has changed. Remaining regions are illustrative and cannot be selected for allocation.", projected) : null;
    }
    private static PartitionLayout Unknown(List<StoragePartition> partitions) => new(false,
        "Physical positioning is unavailable. Reported capacities are shown with minimum widths for tiny partitions; no gaps are inferred.",
        partitions.Select(p => new PartitionSegment(p, p.OffsetBytes ?? 0, p.Device.SizeBytes ?? 0)).ToList());
}
