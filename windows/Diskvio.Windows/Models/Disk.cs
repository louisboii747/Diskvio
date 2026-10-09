using System.Globalization;
using System.Text.Json.Serialization;

namespace Diskvio.Windows.Models;

// This matches diskvio_core::Disk. Missing partition fields never mean "unallocated".
public sealed record Disk
{
    [JsonPropertyName("number")]
    public required uint Number { get; init; }
    [JsonPropertyName("name")]
    public required string Name { get; init; }
    [JsonPropertyName("size_bytes")]
    public required ulong SizeBytes { get; init; }
    [JsonPropertyName("bus_type")]
    public required string BusType { get; init; }
    [JsonPropertyName("partition_style")]
    public required string PartitionStyle { get; init; }

    [JsonIgnore] public string DisplayName => DisplayValue(Name);
    [JsonIgnore] public string Identifier => $@"\\.\PhysicalDrive{Number}";
    [JsonIgnore] public string Capacity => FormatCapacity(SizeBytes);
    [JsonIgnore] public string ExactCapacity => $"{SizeBytes.ToString("N0", CultureInfo.CurrentCulture)} bytes";
    [JsonIgnore] public string Connection => DisplayValue(BusType);
    [JsonIgnore] public string Scheme => DisplayValue(PartitionStyle);
    [JsonIgnore] public string ListSummary => $"Disk {Number} · {Capacity} · {Connection}";
    [JsonIgnore] public string AccessibleName => $"{DisplayName}, {ListSummary}";

    public override string ToString() => AccessibleName;

    private static string DisplayValue(string value) => string.IsNullOrWhiteSpace(value) ? "Not reported" : value;

    // Decimal units match drive manufacturers and the existing CLI/macOS app.
    public static string FormatCapacity(ulong bytes)
    {
        string[] units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
        double amount = bytes;
        var unit = 0;
        while (amount >= 1000 && unit < units.Length - 1)
        {
            amount /= 1000;
            unit++;
        }
        return $"{amount.ToString(unit == 0 ? "N0" : "N2", CultureInfo.CurrentCulture)} {units[unit]}";
    }
}
