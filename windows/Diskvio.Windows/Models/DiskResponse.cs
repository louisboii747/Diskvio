using System.Text.Json.Serialization;

namespace Diskvio.Windows.Models;

internal sealed class DiskResponse
{
    [JsonPropertyName("status")]
    public required string Status { get; init; }
    [JsonPropertyName("disks")]
    public List<Disk>? Disks { get; init; }
    [JsonPropertyName("message")]
    public string? Message { get; init; }
}
