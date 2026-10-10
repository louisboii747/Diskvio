using System.Text.Json.Serialization;

namespace Diskvio.Windows.Models;

[JsonConverter(typeof(DiskActionConverter))]
public enum DiskAction
{
    [JsonStringEnumMemberName("mount")] Mount,
    [JsonStringEnumMemberName("unmount")] Unmount,
    [JsonStringEnumMemberName("eject")] Eject,
    [JsonStringEnumMemberName("rename_volume")] RenameVolume,
    [JsonStringEnumMemberName("set_drive_letter")] SetDriveLetter
}

public sealed class DiskActionConverter() : JsonStringEnumConverter<DiskAction>(allowIntegerValues: false);

public sealed record StorageInventory
{
    [JsonPropertyName("disks")] public required List<StorageDisk> Disks { get; init; }
    [JsonPropertyName("apfs_containers")] public required List<System.Text.Json.JsonElement> ApfsContainers { get; init; }
    [JsonPropertyName("warnings")] public required List<string> Warnings { get; init; }
    [JsonPropertyName("unattached_volumes")] public List<StorageVolume> UnattachedVolumes { get; init; } = [];
}

public sealed record StorageDisk
{
    [JsonPropertyName("device")] public required StorageDevice Device { get; init; }
    [JsonPropertyName("number")] public required uint Number { get; init; }
    [JsonPropertyName("partition_scheme")] public required string PartitionScheme { get; init; }
    [JsonPropertyName("connection_type")] public string? ConnectionType { get; init; }
    [JsonPropertyName("internal")] public bool? Internal { get; init; }
    [JsonPropertyName("removable")] public bool? Removable { get; init; }
    [JsonPropertyName("ejectable")] public bool? Ejectable { get; init; }
    [JsonPropertyName("disk_uuid")] public string? DiskUuid { get; init; }
    [JsonPropertyName("mbr_signature")] public uint? MbrSignature { get; init; }
    [JsonPropertyName("partitions")] public required List<StoragePartition> Partitions { get; init; }
}

public sealed record StoragePartition
{
    [JsonPropertyName("device")] public required StorageDevice Device { get; init; }
    [JsonPropertyName("number")] public uint? Number { get; init; }
    [JsonPropertyName("role")] public string? Role { get; init; }
    [JsonPropertyName("content_type")] public string? ContentType { get; init; }
    [JsonPropertyName("offset_bytes")] public ulong? OffsetBytes { get; init; }
    [JsonPropertyName("gpt_type")] public string? GptType { get; init; }
    [JsonPropertyName("mbr_type")] public ushort? MbrType { get; init; }
    [JsonPropertyName("active")] public bool? Active { get; init; }
    [JsonPropertyName("shadow_copy")] public bool? ShadowCopy { get; init; }
    [JsonPropertyName("no_default_drive_letter")] public bool? NoDefaultDriveLetter { get; init; }
    [JsonPropertyName("volumes")] public List<StorageVolume> Volumes { get; init; } = [];
}

public sealed record StorageVolume
{
    [JsonPropertyName("device")] public required StorageDevice Device { get; init; }
    [JsonPropertyName("label")] public string? Label { get; init; }
}

public sealed record StorageDevice
{
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("name")] public required string Name { get; init; }
    [JsonPropertyName("size_bytes")] public ulong? SizeBytes { get; init; }
    [JsonPropertyName("used_bytes")] public ulong? UsedBytes { get; init; }
    [JsonPropertyName("available_bytes")] public ulong? AvailableBytes { get; init; }
    [JsonPropertyName("filesystem")] public StorageFilesystem? Filesystem { get; init; }
    [JsonPropertyName("volume_uuid")] public string? VolumeUuid { get; init; }
    [JsonPropertyName("partition_uuid")] public string? PartitionUuid { get; init; }
    [JsonPropertyName("stable_id")] public string? StableId { get; init; }
    [JsonPropertyName("identity_token")] public string? IdentityToken { get; init; }
    [JsonPropertyName("drive_letter")] public string? DriveLetter { get; init; }
    [JsonPropertyName("mount_point")] public string? MountPoint { get; init; }
    [JsonPropertyName("mount_points")] public List<string> MountPoints { get; init; } = [];
    [JsonPropertyName("actions")] public List<DiskAction> Actions { get; init; } = [];
    [JsonPropertyName("safety")] public StorageSafety Safety { get; init; } = new();
    [JsonPropertyName("health_status")] public string? HealthStatus { get; init; }
    [JsonPropertyName("operational_status")] public List<string> OperationalStatus { get; init; } = [];
}

public sealed record StorageFilesystem
{
    [JsonPropertyName("name")] public required string Name { get; init; }
    [JsonPropertyName("kind")] public required string Kind { get; init; }
}

public sealed record StorageSafety
{
    [JsonPropertyName("system")] public bool? System { get; init; }
    [JsonPropertyName("boot")] public bool? Boot { get; init; }
    [JsonPropertyName("recovery")] public bool? Recovery { get; init; }
    [JsonPropertyName("hidden")] public bool? Hidden { get; init; }
    [JsonPropertyName("read_only")] public bool? ReadOnly { get; init; }
    [JsonPropertyName("offline")] public bool? Offline { get; init; }
    [JsonPropertyName("page_file")] public bool? PageFile { get; init; }
}

public sealed record DiskOperationRequest
{
    [JsonPropertyName("action")] public required DiskAction Action { get; init; }
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("expected_identity")] public required string ExpectedIdentity { get; init; }
    [JsonPropertyName("volume_label")] public string? VolumeLabel { get; init; }
    [JsonPropertyName("drive_letter")] public string? DriveLetter { get; init; }
    [JsonPropertyName("expected_mount_points")] public List<string>? ExpectedMountPoints { get; init; }
}

public sealed record DiskOperationOutcome
{
    [JsonPropertyName("action")] public required DiskAction Action { get; init; }
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("message")] public required string Message { get; init; }
}

public sealed record OperationCapabilities
{
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("device_kind")] public required string DeviceKind { get; init; }
    [JsonPropertyName("identity_token")] public string? IdentityToken { get; init; }
    [JsonPropertyName("actions")] public required List<DiskAction> Actions { get; init; }
    [JsonPropertyName("unsupported_reason")] public string? UnsupportedReason { get; init; }
    [JsonPropertyName("label_max_length")] public int? LabelMaxLength { get; init; }
    [JsonPropertyName("limitations")] public List<string> Limitations { get; init; } = [];
}

public sealed record OperationValidation
{
    [JsonPropertyName("valid")] public required bool Valid { get; init; }
    [JsonPropertyName("action")] public required DiskAction Action { get; init; }
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("expected_identity")] public required string ExpectedIdentity { get; init; }
}

public sealed record BackendOperationError
{
    [JsonPropertyName("code")] public required string Code { get; init; }
    [JsonPropertyName("message")] public required string Message { get; init; }
    [JsonPropertyName("platform_code")] public long? PlatformCode { get; init; }
}

internal sealed record StorageResponse
{
    [JsonPropertyName("status")] public required string Status { get; init; }
    [JsonPropertyName("inventory")] public StorageInventory? Inventory { get; init; }
    [JsonPropertyName("operation")] public DiskOperationOutcome? Operation { get; init; }
    [JsonPropertyName("capabilities")] public OperationCapabilities? Capabilities { get; init; }
    [JsonPropertyName("validation")] public OperationValidation? Validation { get; init; }
    [JsonPropertyName("message")] public string? Message { get; init; }
    [JsonPropertyName("error")] public BackendOperationError? Error { get; init; }
}
