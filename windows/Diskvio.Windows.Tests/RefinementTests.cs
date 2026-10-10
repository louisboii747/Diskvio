using System.Text.Json;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;

namespace Diskvio.Windows.Tests;

public sealed class RefinementTests
{
    private static StorageDevice Device(string id) => new() { Identifier = id, Name = id, SizeBytes = 1000 };
    private static StoragePartition Partition(string? role = null, string? label = null) => new()
    {
        Device = Device("PhysicalDrive3Partition2"), Number = 2, Role = role, OffsetBytes = 10,
        Volumes = label is null ? [] : [new() { Device = Device("volume"), Label = label }]
    };

    [Fact] public void NamingPrefersLabelsThenRolesThenPartitionNumbers()
    {
        Assert.Equal("Work files", PartitionPresentation.Name(Partition("basic_data", "Work files")));
        Assert.Equal("EFI System Partition", PartitionPresentation.Name(Partition("efi_system")));
        Assert.Equal("Recovery Partition", PartitionPresentation.Name(Partition("recovery", " ")));
        Assert.Equal("Microsoft Reserved", PartitionPresentation.Name(Partition("microsoft_reserved")));
        Assert.Equal("Basic Data", PartitionPresentation.Name(Partition("basic_data")));
        Assert.Equal("Partition 2", PartitionPresentation.Name(Partition("unknown")));
        Assert.Equal("Partition", PartitionPresentation.Name(Partition() with { Number = null }));
        Assert.Equal("Recovery Partition", PartitionPresentation.Name(Partition() with { ContentType = "Recovery" }));
    }

    [Theory]
    [InlineData("{C12A7328-F81F-11D2-BA4B-00A0C93EC93B}", "efi_system")]
    [InlineData("de94bba4-06d1-4d40-a16a-bfd50179d6ac", "recovery")]
    [InlineData("e3c9e316-0b5c-4db8-817d-f92df00215ae", "microsoft_reserved")]
    [InlineData("ebd0a0a2-b9e5-4433-87c0-68b6b72699c7", "basic_data")]
    [InlineData("other", "unknown")]
    public void LegacyContractsDeriveOnlyKnownRoles(string guid, string expected)
        => Assert.Equal(expected, PartitionPresentation.Role(Partition() with { GptType = guid }));

    [Fact] public void TechnicalIdentifiersRemainSecondaryAndMissingMetadataStaysUnknown()
    {
        var p = Partition(); var node = new StorageNode(p.Device, "Partition", Partition: p);
        Assert.Equal("Partition 2", node.Name); Assert.Contains(p.Device.Identifier, node.AccessibleName);
        Assert.Equal("Not reported", node.Filesystem); Assert.Equal("—", node.Location);
        Assert.Equal("Unlabelled volume", new StorageNode(Device(@"\\?\Volume{guid}\"), "Volume").Name);
        Assert.Equal("Volume (E:)", new StorageNode(Device("raw") with { DriveLetter = "E" }, "Volume").Name);
    }

    [Fact] public void TinyPartitionsRemainReachableWithoutClaimingFreeSpace()
    {
        var disk = Inventory().Disks[0] with
        {
            Device = Device("disk") with { SizeBytes = 1_000_000 },
            Partitions = [Partition("efi_system") with { Device = Device("small") with { SizeBytes = 1 }, OffsetBytes = 1 },
                Partition("basic_data") with { Device = Device("large") with { SizeBytes = 999_998 }, OffsetBytes = 2 }]
        };
        var map = PartitionLayout.Create(disk); var widths = map.VisualWidths(500);
        Assert.True(map.IsPhysical); Assert.Equal(500, widths.Sum(), 6);
        Assert.True(widths[1] >= 36); Assert.Equal("efi_system", map.Segments[1].Category);
        Assert.Contains("not verified", map.Explanation);
        Assert.Equal("unmapped", map.Segments[0].Category);
        Assert.DoesNotContain("Unallocated", map.Segments[0].Label);
        var dense = new PartitionLayout(false, "", Enumerable.Range(0, 40).Select(_ => new PartitionSegment(Partition(), 0, 1)).ToList());
        Assert.All(dense.VisualWidths(400), w => Assert.True(w >= 36));
        Assert.Equal(1440, dense.VisualWidths(400).Sum());
    }

    [Fact] public async Task PartitionActionsResolveOnlyOneVolumeAndNeedMatchingCapabilities()
    {
        var (vm, service) = await Ready();
        Assert.True(vm.CanRename); Assert.True(vm.CanSetDriveLetter);
        Assert.Equal("volume", vm.ActionTarget?.Device.Identifier);
        service.Token = "stale"; await vm.QueryCapabilitiesAsync(); Assert.False(vm.CanRename);
        service.Token = "token";
        var disk = service.Value.Disks[0]; var partition = disk.Partitions[0];
        service.Value = service.Value with { Disks = [disk with { Partitions = [partition with { Volumes = [.. partition.Volumes, new() { Device = Device("second") }] }] }] };
        await vm.RefreshAsync(); vm.SelectPartition(vm.SelectedDisk!.Disk!.Partitions[0]); await vm.QueryCapabilitiesAsync();
        Assert.False(vm.CanRename); Assert.False(vm.CanSetDriveLetter);
    }

    [Fact] public async Task ProtectedUnknownInternalAndPartialStatesBlockNewActions()
    {
        foreach (var safety in new[] { Safe() with { System = true }, Safe() with { Boot = true }, Safe() with { Recovery = true }, Safe() with { PageFile = true }, Safe() with { ReadOnly = true }, Safe() with { Hidden = true }, Safe() with { Offline = true }, Safe() with { Boot = null } })
        {
            var service = new Fake(); var disk = service.Value.Disks[0]; var p = disk.Partitions[0];
            service.Value = service.Value with { Disks = [disk with { Partitions = [p with { Volumes = [p.Volumes[0] with { Device = p.Volumes[0].Device with { Safety = safety } }] }] }] };
            var (vm, _) = await Ready(service); Assert.False(vm.CanRename); Assert.False(vm.CanSetDriveLetter);
            await vm.ExecuteWithParametersAsync(DiskAction.RenameVolume, (_, _) => Task.FromResult<OperationInput?>(new("New")));
            Assert.Empty(service.Requests);
        }
        var (partial, fake) = await Ready(); fake.Value = fake.Value with { Warnings = ["Partial"] };
        await partial.RefreshAsync(); Assert.False(partial.CanRename);
        fake.Value = fake.Value with { Warnings = [], Disks = [fake.Value.Disks[0] with { Internal = true }] };
        await partial.RefreshAsync(); Assert.False(partial.CanSetDriveLetter);
    }

    [Fact] public async Task ParameterizedChangesRequireConfirmationValidateAndRefresh()
    {
        var (vm, fake) = await Ready();
        await vm.ExecuteWithParametersAsync(DiskAction.RenameVolume, (_, _) => Task.FromResult<OperationInput?>(null));
        Assert.Empty(fake.Requests);
        await vm.ExecuteWithParametersAsync(DiskAction.RenameVolume, (_, _) => Task.FromResult<OperationInput?>(new("New label")));
        Assert.Equal("New label", Assert.Single(fake.Requests).VolumeLabel);
        Assert.NotNull(fake.Requests[0].ExpectedMountPoints);
        Assert.Equal(fake.Requests[0], Assert.Single(fake.Validations));
        Assert.True(vm.HasResult); Assert.Equal("PhysicalDrive3Partition2", vm.SelectedNode?.Device.Identifier);
        await vm.ExecuteWithParametersAsync(DiskAction.SetDriveLetter, (_, _) => Task.FromResult<OperationInput?>(new(DriveLetter: "H")));
        Assert.Equal("H", fake.Requests[1].DriveLetter); Assert.Equal(3, fake.Discoveries);
    }

    [Fact] public async Task DisappearingTargetAndBackendRejectionRefreshWithoutStaleSelection()
    {
        var (vm, fake) = await Ready(); fake.Reject = true;
        await vm.ExecuteWithParametersAsync(DiskAction.RenameVolume, (_, _) => Task.FromResult<OperationInput?>(new("New label")));
        Assert.True(vm.HasError); Assert.False(vm.HasResult); Assert.Equal(2, fake.Discoveries);
        fake.Value = fake.Value with { Disks = [] }; await vm.RefreshAsync();
        Assert.Null(vm.SelectedNode); Assert.Null(vm.Layout); Assert.False(vm.CanRename); Assert.False(vm.CanSetDriveLetter);
    }

    [Fact] public async Task RefreshRestoresStableVolumeAfterDiskAndIdentifierRenumbering()
    {
        var (vm, fake) = await Ready();
        vm.SelectedNode = vm.SelectedDisk!.Children[0].Children[0];
        var disk = fake.Value.Disks[0]; var partition = disk.Partitions[0]; var volume = partition.Volumes[0];
        fake.Value = fake.Value with { Disks = [disk with { Number = 8, Device = disk.Device with { Identifier = "new-disk-number" },
            Partitions = [partition with { Device = partition.Device with { Identifier = "new-partition-number" },
                Volumes = [volume with { Device = volume.Device with { Identifier = "renumbered-volume" } }] }] }] };
        await vm.RefreshAsync();
        Assert.Equal((uint)8, vm.SelectedDisk?.Disk?.Number);
        Assert.Equal("renumbered-volume", vm.SelectedNode?.Device.Identifier);
        Assert.True(vm.CanRename);
    }

    [Fact] public void AdditiveContractPreservesNewActionsMetadataAndParameters()
    {
        var caps = DiskService.DecodeCapabilities("""{"status":"ok","capabilities":{"identifier":"volume","device_kind":"volume","actions":["rename_volume","set_drive_letter"],"label_max_length":32,"limitations":["No partition editing"]}}""");
        Assert.Contains(DiskAction.RenameVolume, caps.Actions); Assert.Equal(32, caps.LabelMaxLength);
        var json = JsonSerializer.Serialize(new DiskOperationRequest { Action = DiskAction.RenameVolume, Identifier = "volume", ExpectedIdentity = "token", VolumeLabel = "Données" });
        Assert.Contains("rename_volume", json); Assert.Equal("Données", JsonDocument.Parse(json).RootElement.GetProperty("volume_label").GetString());
    }

    private static StorageSafety Safe() => new() { System = false, Boot = false, Recovery = false, PageFile = false, Hidden = false, Offline = false, ReadOnly = false };
    private static StorageInventory Inventory() => new()
    {
        Disks = [new() { Number = 3, Device = Device("disk") with { StableId = "disk-stable" }, PartitionScheme = "gpt", Internal = false, ConnectionType = "USB",
            Partitions = [Partition("basic_data") with { Volumes = [new() { Label = "Original", Device = Device("volume") with { IdentityToken = "token", Safety = Safe(), Actions = [DiskAction.RenameVolume, DiskAction.SetDriveLetter] } }] }] }], ApfsContainers = [], Warnings = []
    };
    private static async Task<(StorageWorkspaceViewModel, Fake)> Ready(Fake? fake = null)
    {
        fake ??= new(); var vm = new StorageWorkspaceViewModel(fake); await vm.RefreshAsync();
        vm.SelectPartition(vm.SelectedDisk!.Disk!.Partitions[0]); await vm.QueryCapabilitiesAsync(); return (vm, fake);
    }
    private sealed class Fake : IDiskService
    {
        public StorageInventory Value = Inventory(); public string Token = "token"; public bool Reject; public int Discoveries;
        public List<DiskOperationRequest> Requests = [], Validations = [];
        public Task<StorageInventory> InventoryAsync() { Discoveries++; return Task.FromResult(Value); }
        public Task<OperationCapabilities> SupportedOperationsAsync(string id) => Task.FromResult(new OperationCapabilities { Identifier = id, IdentityToken = Token, DeviceKind = "volume", Actions = [DiskAction.RenameVolume, DiskAction.SetDriveLetter], LabelMaxLength = 32 });
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest r) { Validations.Add(r); return Reject ? Task.FromException<OperationValidation>(new DiskServiceException("Backend rejected target")) : Task.FromResult(new OperationValidation { Valid = true, Action = r.Action, Identifier = r.Identifier, ExpectedIdentity = r.ExpectedIdentity }); }
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest r) { Requests.Add(r); return Task.FromResult(new DiskOperationOutcome { Action = r.Action, Identifier = r.Identifier, Message = "Test result" }); }
        public Task<IReadOnlyList<Disk>> ListDisksAsync() => throw new NotSupportedException();
    }
}
