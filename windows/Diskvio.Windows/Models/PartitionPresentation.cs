namespace Diskvio.Windows.Models;

// Presentation only: never used to authorize an operation or identify free space.
public static class PartitionPresentation
{
    public static string Role(StoragePartition p) => p.Role ?? p.GptType?.Trim('{', '}').ToLowerInvariant() switch
    {
        "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" => "efi_system",
        "de94bba4-06d1-4d40-a16a-bfd50179d6ac" => "recovery",
        "e3c9e316-0b5c-4db8-817d-f92df00215ae" => "microsoft_reserved",
        "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7" => "basic_data",
        _ => p.MbrType switch { 0xef => "efi_system", 0x27 => "recovery", 1 or 4 or 6 or 7 or 11 or 12 or 14 => "basic_data", _ => p.ContentType?.ToLowerInvariant() switch { "recovery" => "recovery", "reserved" => "microsoft_reserved", "basic" or "ifs" => "basic_data", _ => "unknown" } }
    };

    public static string? RoleName(StoragePartition p) => Role(p) switch
    {
        "efi_system" => "EFI System Partition", "recovery" => "Recovery Partition",
        "microsoft_reserved" => "Microsoft Reserved", "basic_data" => "Basic Data", _ => null
    };

    public static string Name(StoragePartition p) => p.Volumes.Select(v => v.Label).FirstOrDefault(v => !string.IsNullOrWhiteSpace(v))?.Trim()
        ?? RoleName(p) ?? (p.Number is { } number ? $"Partition {number}" : "Partition");

    public static string VolumeName(StorageVolume? volume, StorageDevice device) => !string.IsNullOrWhiteSpace(volume?.Label)
        ? volume.Label.Trim() : !string.IsNullOrWhiteSpace(device.Name) && device.Name != device.Identifier && !device.Name.StartsWith(@"\\?\", StringComparison.Ordinal)
            ? device.Name : device.DriveLetter is { } letter ? $"Volume ({letter}:)" : "Unlabelled volume";

    public static string Filesystems(StoragePartition p) => string.Join(", ", p.Volumes.Select(v => v.Device.Filesystem?.Name).Append(p.Device.Filesystem?.Name).Where(s => !string.IsNullOrWhiteSpace(s)).Distinct());
}
