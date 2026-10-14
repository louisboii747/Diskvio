using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;

namespace Diskvio.Windows.Tests;

public sealed class BrowserTests
{
    private static readonly string[] ExpectedGroups = ["Internal", "Removable", "Other devices"];
    private static readonly string[] SortedIdentifiers = ["large", "small", "child"];
    private static readonly string[] SearchIdentifiers = ["partition", "volume"];
    private static StorageDevice Device(string id, ulong size = 1000) => new() { Identifier = id, Name = id, SizeBytes = size };
    private static StorageDisk Disk(uint number, bool? internalDisk, bool? removable = false) => new()
    {
        Device = Device($"disk{number}"), Number = number, PartitionScheme = "gpt", Internal = internalDisk,
        Removable = removable, Model = "Example model", Partitions = []
    };

    [Fact] public async Task SearchIncludesNestedVolumesAndFiltersUseReportedLocation()
    {
        var disk = Disk(2, false, true) with { Partitions = [new() { Device = Device("partition"), Number = 1,
            Volumes = [new() { Device = Device("volume") with { MountPoint = "E:\\" }, Label = "Project archive" }] }] };
        using var vm = new StorageWorkspaceViewModel(new Fake([Disk(0, true), disk, Disk(3, null, null)]));
        await vm.RefreshAsync();
        Assert.Equal(ExpectedGroups, vm.DeviceGroups.Select(g => g.Name));
        vm.SearchText = "project ARCHIVE";
        Assert.Equal(SearchIdentifiers, Assert.Single(vm.DeviceGroups).Select(n => n.Device.Identifier));
        vm.SelectBrowserNode(Assert.Single(vm.DeviceGroups.SelectMany(g => g), n => n.Kind == "Volume"));
        Assert.Equal("disk2", vm.SelectedDisk?.Device.Identifier);
        Assert.Equal("volume", vm.SelectedNode?.Device.Identifier);
        Assert.Equal("partition", vm.SelectedMapId);
        vm.SearchText = " E:\\ ";
        Assert.Equal("volume", Assert.Single(Assert.Single(vm.DeviceGroups)).Device.Identifier);
        vm.SearchText = ""; vm.DeviceFilter = "External drives";
        Assert.Equal("disk2", Assert.Single(Assert.Single(vm.DeviceGroups)).Device.Identifier);
        vm.DeviceFilter = "Internal drives";
        Assert.Equal("disk0", Assert.Single(Assert.Single(vm.DeviceGroups)).Device.Identifier);
        vm.SearchText = "not connected";
        Assert.True(vm.HasNoMatches);
        Assert.NotNull(vm.SelectedDisk);
    }

    [Fact] public async Task SortingPreservesVolumeAssociationAndSelectedMapTarget()
    {
        var disk = Disk(2, false) with { Partitions = [new() { Device = Device("small", 100), Number = 1,
            Volumes = [new() { Device = Device("child"), Label = "Archive" }] }, new() { Device = Device("large", 900), Number = 2 }] };
        using var vm = new StorageWorkspaceViewModel(new Fake([disk]));
        await vm.RefreshAsync();
        vm.SelectPartition(disk.Partitions[0]);
        vm.PartitionSort = "Largest first";
        Assert.Equal(SortedIdentifiers, vm.Children.Select(n => n.Device.Identifier));
        Assert.Equal("small", vm.SelectedMapId);
        Assert.Equal("small", vm.SelectedNode?.Device.Identifier);
    }

    [Fact] public void AmbiguousPartitionUsageIsNeverInferredFromOneOfSeveralVolumes()
    {
        var volume = new StorageVolume { Device = Device("volume") with { UsedBytes = 400, AvailableBytes = 600 } };
        var partition = new StoragePartition { Device = Device("partition"), Volumes = [volume] };
        var node = new StorageNode(partition.Device, "Partition", Partition: partition);
        Assert.Equal(400ul, node.UsedBytes);
        Assert.Equal(600ul, node.AvailableBytes);
        var ambiguous = partition with { Volumes = [volume, volume with { Device = Device("second") }] };
        Assert.Null(new StorageNode(ambiguous.Device, "Partition", Partition: ambiguous).UsedBytes);
    }

    [Theory]
    [InlineData("NTFS", "NTFS")]
    [InlineData("APFS", "APFS")]
    [InlineData("FAT32", "FAT")]
    [InlineData("exFAT", "ExFAT")]
    [InlineData("ext4", "Ext")]
    [InlineData("newfs", "Unknown")]
    public void PaletteUsesFilesystemMetadataInsteadOfVolumeLabels(string format, string expected)
    {
        var p = new StoragePartition { Device = Device("partition") with { Name = "APFS backup", Filesystem = new() { Name = format, Kind = "other" } } };
        Assert.Equal(expected, PartitionPresentation.Tone(p));
    }

    [Fact] public void UnknownPartitionCapacityRemainsSelectableWithoutReportingZeroBytes()
    {
        var partition = new StoragePartition { Device = Device("unknown") with { SizeBytes = null } };
        var layout = PartitionLayout.Create(Disk(0, false) with { Partitions = [partition] });
        var segment = Assert.Single(layout.Segments);
        Assert.Equal("unknown", segment.SelectionId);
        Assert.Contains("Capacity not reported", segment.Description);
        Assert.DoesNotContain("(0 bytes)", segment.Description);
        Assert.True(Assert.Single(layout.VisualWidths(500)) >= 36);
    }

    private sealed class Fake(List<StorageDisk> disks) : IDiskService
    {
        public Task<StorageInventory> InventoryAsync() => Task.FromResult(new StorageInventory { Disks = disks, ApfsContainers = [], Warnings = [] });
        public Task<IReadOnlyList<Models.Disk>> ListDisksAsync() => throw new NotSupportedException();
        public Task<OperationCapabilities> SupportedOperationsAsync(string identifier) => Task.FromResult(new OperationCapabilities { Identifier = identifier, DeviceKind = "physical_disk", Actions = [] });
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest request) => throw new NotSupportedException();
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request) => throw new NotSupportedException();
    }
}
