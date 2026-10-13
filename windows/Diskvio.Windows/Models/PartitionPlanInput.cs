using System.Globalization;
using System.Numerics;

namespace Diskvio.Windows.Models;

public static class PartitionPlanInput
{
    public const string BasicDataType = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";

    public static bool TryBytes(string text, string unit, out ulong bytes)
    {
        bytes = 0;
        var multiplier = unit switch { "MiB" => 1_048_576ul, "GiB" => 1_073_741_824ul, "bytes" => 1ul, _ => 0ul };
        var parts = text.Split('.');
        if (text.Length > 128 || multiplier == 0 || parts.Length > 2 || parts.Any(p => p.Length == 0 || p.Any(c => c is < '0' or > '9')) ||
            !ulong.TryParse(parts[0], NumberStyles.None, CultureInfo.InvariantCulture, out var whole)) return false;
        try
        {
            bytes = checked(whole * multiplier);
            if (parts.Length == 1) return true;
            if (!BigInteger.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, out var fraction)) return false;
            var divisor = BigInteger.Pow(10, parts[1].Length);
            var numerator = fraction * multiplier;
            if (numerator % divisor != 0) return false;
            bytes = checked(bytes + (ulong)(numerator / divisor));
            return true;
        }
        catch (OverflowException) { bytes = 0; return false; }
    }

    public static string RegionId(AllocatableRegion region) => $"region:{region.Identity.StartOffsetBytes}:{region.Identity.EndOffsetBytes}";

    public static PartitionTarget Target(PlanningPartition partition) => new()
    {
        PartitionId = partition.PartitionId ?? throw new InvalidOperationException("Partition identity is unavailable. Inspect the layout again."),
        Number = partition.Number ?? throw new InvalidOperationException("Partition number is unavailable."),
        OffsetBytes = partition.OffsetBytes ?? throw new InvalidOperationException("Partition offset is unavailable."),
        SizeBytes = partition.SizeBytes ?? throw new InvalidOperationException("Partition size is unavailable."),
        GptType = partition.GptType ?? throw new InvalidOperationException("Partition type is unavailable.")
    };

    public static string Issues(PlanningValidation validation) => string.Join(Environment.NewLine,
        validation.Issues.Select(i => $"{i.Message} ({i.Code}{(i.PlatformCode is { } code ? $", Windows {code}" : "")})"));

    public static string Status(PlanningValidation validation)
    {
        var codes = validation.Issues.Select(i => i.Code).ToHashSet(StringComparer.Ordinal);
        if (codes.Overlaps(["device_disappeared"])) return "Device unavailable";
        if (codes.Overlaps(["stale_device_identity", "changed_partition_layout"])) return "Stale identity or layout";
        if (codes.Overlaps(["discovery_failed", "permission_denied", "internal_error"])) return "Backend discovery failed";
        if (codes.Overlaps(["unsupported_operation", "unsupported_platform"])) return "Unsupported planning operation";
        if (codes.Overlaps(["protected_target", "missing_metadata", "ambiguous_identity"])) return "Protected or unverified target";
        return validation.Valid ? "Technically valid preview" : "Preview rejected";
    }
}
