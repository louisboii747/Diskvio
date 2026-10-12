using System.Text.Json.Serialization;

namespace Diskvio.Windows.Models;

public sealed record PhysicalDiskIdentity
{
    [JsonPropertyName("unique_id")] public required string UniqueId { get; init; }
    [JsonPropertyName("unique_id_format")] public required ushort UniqueIdFormat { get; init; }
    [JsonPropertyName("serial_number")] public required string SerialNumber { get; init; }
}

public sealed record DiskLayoutSnapshot
{
    [JsonPropertyName("version")] public required uint Version { get; init; }
    [JsonPropertyName("token")] public required string Token { get; init; }
}

public sealed record AllocatableRegionIdentity
{
    [JsonPropertyName("start_offset_bytes")] public required ulong StartOffsetBytes { get; init; }
    [JsonPropertyName("end_offset_bytes")] public required ulong EndOffsetBytes { get; init; }
}

public sealed record TestDiskDesignation
{
    [JsonPropertyName("identity")] public required PhysicalDiskIdentity Identity { get; init; }
    [JsonPropertyName("purpose")] public string Purpose { get; } = "expendable_external_test_disk";
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "mode")]
[JsonDerivedType(typeof(DiscoverPartitionLayoutRequest), "discover")]
[JsonDerivedType(typeof(CreatePartitionPlanRequest), "create")]
[JsonDerivedType(typeof(DeletePartitionPlanRequest), "delete")]
public abstract record PartitionPlanningRequest
{
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
}

public sealed record DiscoverPartitionLayoutRequest : PartitionPlanningRequest;

public abstract record PartitionPlanRequest : PartitionPlanningRequest
{
    [JsonPropertyName("expected_identity")] public required PhysicalDiskIdentity ExpectedIdentity { get; init; }
    [JsonPropertyName("expected_layout")] public required DiskLayoutSnapshot ExpectedLayout { get; init; }
    [JsonPropertyName("test_disk")] public required TestDiskDesignation TestDisk { get; init; }
}

public sealed record CreatePartitionPlanRequest : PartitionPlanRequest
{
    [JsonPropertyName("region")] public required AllocatableRegionIdentity Region { get; init; }
    [JsonPropertyName("requested_size_bytes")] public required ulong RequestedSizeBytes { get; init; }
    [JsonPropertyName("requested_start_offset_bytes")] public ulong? RequestedStartOffsetBytes { get; init; }
    [JsonPropertyName("required_alignment_bytes")] public required ulong RequiredAlignmentBytes { get; init; }
    [JsonPropertyName("partition_table")] public required string PartitionTable { get; init; }
    [JsonPropertyName("proposed_gpt_type")] public required string ProposedGptType { get; init; }
}

public sealed record PartitionTarget
{
    [JsonPropertyName("partition_id")] public required string PartitionId { get; init; }
    [JsonPropertyName("number")] public required uint Number { get; init; }
    [JsonPropertyName("offset_bytes")] public required ulong OffsetBytes { get; init; }
    [JsonPropertyName("size_bytes")] public required ulong SizeBytes { get; init; }
    [JsonPropertyName("gpt_type")] public required string GptType { get; init; }
}

public sealed record DeletePartitionPlanRequest : PartitionPlanRequest
{
    [JsonPropertyName("partition")] public required PartitionTarget Partition { get; init; }
}

public sealed record PlanningValidation
{
    [JsonPropertyName("valid")] public required bool Valid { get; init; }
    [JsonPropertyName("issues")] public required List<BackendOperationError> Issues { get; init; }
}

public sealed record GptGeometry
{
    [JsonPropertyName("disk_guid")] public required string DiskGuid { get; init; }
    [JsonPropertyName("disk_size_bytes")] public required ulong DiskSizeBytes { get; init; }
    [JsonPropertyName("usable_start_bytes")] public required ulong UsableStartBytes { get; init; }
    [JsonPropertyName("usable_end_bytes")] public required ulong UsableEndBytes { get; init; }
    [JsonPropertyName("logical_sector_bytes")] public required uint LogicalSectorBytes { get; init; }
    [JsonPropertyName("physical_sector_bytes")] public required uint PhysicalSectorBytes { get; init; }
    [JsonPropertyName("sector_alignment_offset_bytes")] public required uint SectorAlignmentOffsetBytes { get; init; }
    [JsonPropertyName("max_partition_count")] public required uint MaxPartitionCount { get; init; }
}

public sealed record PartitionDiskSafety
{
    [JsonPropertyName("external")] public bool? External { get; init; }
    [JsonPropertyName("system")] public bool? System { get; init; }
    [JsonPropertyName("boot")] public bool? Boot { get; init; }
    [JsonPropertyName("boot_from_disk")] public bool? BootFromDisk { get; init; }
    [JsonPropertyName("read_only")] public bool? ReadOnly { get; init; }
    [JsonPropertyName("offline")] public bool? Offline { get; init; }
    [JsonPropertyName("clustered")] public bool? Clustered { get; init; }
    [JsonPropertyName("storage_pool")] public bool? StoragePool { get; init; }
    [JsonPropertyName("dynamic")] public bool? Dynamic { get; init; }
}

public sealed record PartitionSafety
{
    [JsonPropertyName("flags")] public required StorageSafety Flags { get; init; }
    [JsonPropertyName("active")] public bool? Active { get; init; }
    [JsonPropertyName("shadow_copy")] public bool? ShadowCopy { get; init; }
    [JsonPropertyName("encrypted")] public bool? Encrypted { get; init; }
    [JsonPropertyName("hosts_windows")] public bool? HostsWindows { get; init; }
    [JsonPropertyName("volumes_complete")] public bool? VolumesComplete { get; init; }
}

public sealed record PartitionVolumeReference
{
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("mount_points")] public required List<string> MountPoints { get; init; }
}

public sealed record PlanningPartition
{
    [JsonPropertyName("partition_id")] public string? PartitionId { get; init; }
    [JsonPropertyName("number")] public uint? Number { get; init; }
    [JsonPropertyName("offset_bytes")] public ulong? OffsetBytes { get; init; }
    [JsonPropertyName("size_bytes")] public ulong? SizeBytes { get; init; }
    [JsonPropertyName("gpt_type")] public string? GptType { get; init; }
    [JsonPropertyName("attributes")] public ulong? Attributes { get; init; }
    [JsonPropertyName("service_partition")] public bool? ServicePartition { get; init; }
    [JsonPropertyName("name")] public string? Name { get; init; }
    [JsonPropertyName("safety")] public required PartitionSafety Safety { get; init; }
    [JsonPropertyName("volumes")] public required List<PartitionVolumeReference> Volumes { get; init; }
}

public sealed record AllocatableRegion
{
    [JsonPropertyName("parent_identity")] public required PhysicalDiskIdentity ParentIdentity { get; init; }
    [JsonPropertyName("identity")] public required AllocatableRegionIdentity Identity { get; init; }
    [JsonPropertyName("capacity_bytes")] public required ulong CapacityBytes { get; init; }
    [JsonPropertyName("required_alignment_bytes")] public required ulong RequiredAlignmentBytes { get; init; }
    [JsonPropertyName("first_usable_aligned_offset_bytes")] public ulong? FirstUsableAlignedOffsetBytes { get; init; }
    [JsonPropertyName("last_usable_aligned_boundary_bytes")] public ulong? LastUsableAlignedBoundaryBytes { get; init; }
    [JsonPropertyName("maximum_allocatable_bytes")] public required ulong MaximumAllocatableBytes { get; init; }
    [JsonPropertyName("aligned_capacity_bytes")] public required ulong AlignedCapacityBytes { get; init; }
    [JsonPropertyName("partition_table")] public required string PartitionTable { get; init; }
    [JsonPropertyName("validation")] public required PlanningValidation Validation { get; init; }
}

public sealed record CreatePartitionPreview
{
    [JsonPropertyName("parent_identity")] public required PhysicalDiskIdentity ParentIdentity { get; init; }
    [JsonPropertyName("region")] public required AllocatableRegionIdentity Region { get; init; }
    [JsonPropertyName("expected_layout")] public required DiskLayoutSnapshot ExpectedLayout { get; init; }
    [JsonPropertyName("partition_table")] public required string PartitionTable { get; init; }
    [JsonPropertyName("proposed_gpt_type")] public required string ProposedGptType { get; init; }
    [JsonPropertyName("required_alignment_bytes")] public required ulong RequiredAlignmentBytes { get; init; }
    [JsonPropertyName("start_offset_bytes")] public required ulong StartOffsetBytes { get; init; }
    [JsonPropertyName("end_offset_bytes")] public required ulong EndOffsetBytes { get; init; }
    [JsonPropertyName("allocated_size_bytes")] public required ulong AllocatedSizeBytes { get; init; }
    [JsonPropertyName("remaining_region_capacity_bytes")] public required ulong RemainingRegionCapacityBytes { get; init; }
    [JsonPropertyName("remaining_allocatable_capacity_bytes")] public required ulong RemainingAllocatableCapacityBytes { get; init; }
}

public sealed record DeletePartitionPreview
{
    [JsonPropertyName("parent_disk")] public required string ParentDisk { get; init; }
    [JsonPropertyName("parent_identity")] public PhysicalDiskIdentity? ParentIdentity { get; init; }
    [JsonPropertyName("partition")] public required PlanningPartition Partition { get; init; }
    [JsonPropertyName("eligible")] public required bool Eligible { get; init; }
    [JsonPropertyName("validation")] public required PlanningValidation Validation { get; init; }
    [JsonPropertyName("data_loss_warning")] public required string DataLossWarning { get; init; }
}

public sealed record DiskPlanningDiscovery
{
    [JsonPropertyName("identifier")] public required string Identifier { get; init; }
    [JsonPropertyName("name")] public required string Name { get; init; }
    [JsonPropertyName("physical_identity")] public PhysicalDiskIdentity? PhysicalIdentity { get; init; }
    [JsonPropertyName("partition_table")] public required string PartitionTable { get; init; }
    [JsonPropertyName("geometry")] public GptGeometry? Geometry { get; init; }
    [JsonPropertyName("layout_snapshot")] public DiskLayoutSnapshot? LayoutSnapshot { get; init; }
    [JsonPropertyName("safety")] public required PartitionDiskSafety Safety { get; init; }
    [JsonPropertyName("validation")] public required PlanningValidation Validation { get; init; }
    [JsonPropertyName("regions")] public required List<AllocatableRegion> Regions { get; init; }
    [JsonPropertyName("partitions")] public required List<DeletePartitionPreview> Partitions { get; init; }
}

public sealed record PartitionPlanningResult
{
    [JsonPropertyName("api_version")] public required uint ApiVersion { get; init; }
    [JsonPropertyName("preview_only")] public required bool PreviewOnly { get; init; }
    [JsonPropertyName("execution_supported")] public required bool ExecutionSupported { get; init; }
    [JsonPropertyName("validation")] public required PlanningValidation Validation { get; init; }
    [JsonPropertyName("discovery")] public DiskPlanningDiscovery? Discovery { get; init; }
    [JsonPropertyName("create_preview")] public CreatePartitionPreview? CreatePreview { get; init; }
    [JsonPropertyName("delete_preview")] public DeletePartitionPreview? DeletePreview { get; init; }
}
