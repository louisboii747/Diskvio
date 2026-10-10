using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;
namespace Diskvio.Windows.Tests;

public sealed class WorkspaceTests
{
    private static StorageDevice Device(string id, ulong? size = 1000) => new() { Identifier = id, Name = id, SizeBytes = size };
    private static StorageDisk Disk(params StoragePartition[] partitions) => new()
    { Device = Device("disk") with { StableId = "stable" }, Number = 1, PartitionScheme = "gpt", ConnectionType = "USB", Internal = false, Partitions = partitions.ToList() };
    private static StoragePartition Part(ulong? offset, ulong? size, string id = "partition") => new()
    { Device = Device(id, size), OffsetBytes = offset, Volumes = [] };
    [Fact] public void GeometryUsesExactOffsetsAndIncludesOnlyKnownGaps()
    {
        var map = PartitionLayout.Create(Disk(Part(100, 300), Part(500, 500, "second")));
        Assert.True(map.IsPhysical);
        Assert.Equal(new ulong[] { 100, 300, 100, 500 }, map.Segments.Select(s => s.Length));
        Assert.Equal(100ul, map.Segments[1].Offset);
    }
    [Theory] [InlineData(null, 200ul)] [InlineData(0ul, null)] [InlineData(900ul, 200ul)]
    public void IncompleteOrOutOfBoundsGeometryNeverInventsGaps(ulong? offset, ulong? size)
    {
        var map = PartitionLayout.Create(Disk(Part(offset, size)));
        Assert.False(map.IsPhysical); Assert.All(map.Segments, s => Assert.NotNull(s.Partition));
    }
    [Fact] public void OverlappingAndOverflowingGeometryIsRejected()
    {
        Assert.False(PartitionLayout.Create(Disk(Part(0, 600), Part(500, 300, "second"))).IsPhysical);
        Assert.False(PartitionLayout.Create(Disk(Part(ulong.MaxValue, 2))).IsPhysical);
        Assert.False(PartitionLayout.Create(Disk()).IsPhysical);
    }
    [Fact] public async Task RequiresCapabilitiesAndRejectsInternalVolumes()
    {
        var service = new Fake(); var vm = new StorageWorkspaceViewModel(service);
        await vm.RefreshAsync(); vm.SelectedNode = vm.SelectedDisk!.Children[0].Children[0];
        Assert.False(vm.CanUnmount); await vm.QueryCapabilitiesAsync(); Assert.True(vm.CanUnmount);
        service.Inventory = service.Inventory with { Disks = [service.Inventory.Disks[0] with { Internal = true }] };
        await vm.RefreshAsync(); await vm.QueryCapabilitiesAsync(); Assert.False(vm.CanUnmount);
        await vm.ExecuteAsync(DiskAction.Unmount, (_, _) => Task.FromResult(true)); Assert.Equal(0, service.Executions);
    }
    [Fact] public async Task ConfirmationCancellationAndChangedIdentityNeverExecute()
    {
        var (vm, service) = await Ready();
        await vm.ExecuteAsync(DiskAction.Unmount, (_, _) => Task.FromResult(false)); Assert.Equal(0, service.Executions); Assert.True(vm.CanUnmount);
        await vm.QueryCapabilitiesAsync();
        await vm.ExecuteAsync(DiskAction.Unmount, (_, _) => { service.Token = "replacement"; return Task.FromResult(true); });
        Assert.Equal(0, service.Executions); Assert.True(vm.HasError);
    }
    [Fact] public async Task OperationLocksRefreshRevalidatesAndRestoresSelection()
    {
        var (vm, service) = await Ready();
        var confirmation = new TaskCompletionSource<bool>();
        var task = vm.ExecuteAsync(DiskAction.Unmount, (_, _) => confirmation.Task);
        Assert.True(vm.IsBusy); Assert.False(vm.CanUnmount); Assert.False(vm.RefreshCommand.CanExecute(null));
        await vm.ExecuteAsync(DiskAction.Unmount, (_, _) => Task.FromResult(true)); Assert.Equal(0, service.Executions);
        confirmation.SetResult(true); await task;
        Assert.Equal(1, service.Executions); Assert.Equal(1, service.Validations); Assert.Equal("volume", vm.SelectedNode?.Device.Identifier);
        Assert.True(vm.HasResult); Assert.False(vm.IsBusy); Assert.Equal(2, service.Discoveries);
    }
    [Fact] public async Task FailedDiscoveryClearsStaleDevicesAndWarningsBlockActions()
    {
        var (vm, service) = await Ready();
        service.Inventory = service.Inventory with { Warnings = ["Incomplete discovery"] };
        await vm.RefreshAsync(); Assert.False(vm.CanUnmount);
        service.Error = new DiskServiceException("Disconnected"); await vm.RefreshAsync();
        Assert.Empty(vm.Disks); Assert.Null(vm.SelectedNode); Assert.True(vm.HasError);
    }
    [Fact] public async Task LateCapabilitiesCannotEnableAnotherSelection()
    {
        var (vm, service) = await Ready();
        service.Pending = new(); var query = vm.QueryCapabilitiesAsync(); vm.SelectDiskDetails();
        service.Pending.SetResult(service.Caps()); await query; Assert.False(vm.CanUnmount);
    }
    private static async Task<(StorageWorkspaceViewModel, Fake)> Ready()
    {
        var service = new Fake(); var vm = new StorageWorkspaceViewModel(service); await vm.RefreshAsync();
        vm.SelectedNode = vm.SelectedDisk!.Children[0].Children[0]; await vm.QueryCapabilitiesAsync(); return (vm, service);
    }
    private sealed class Fake : IDiskService
    {
        public StorageInventory Inventory { get; set; } = new() { Disks = [Disk(Part(0, 1000) with { Volumes = [new() { Device = Device("volume") with { IdentityToken = "token", Actions = [DiskAction.Unmount] } }] })], ApfsContainers = [], Warnings = [] };
        public string Token { get; set; } = "token";
        public Exception? Error { get; set; }
        public int Executions, Validations, Discoveries;
        public TaskCompletionSource<OperationCapabilities>? Pending;
        public OperationCapabilities Caps() => new() { Identifier = "volume", DeviceKind = "volume", IdentityToken = Token, Actions = [DiskAction.Unmount] };
        public Task<StorageInventory> InventoryAsync() { Discoveries++; return Error is null ? Task.FromResult(Inventory) : Task.FromException<StorageInventory>(Error); }
        public Task<OperationCapabilities> SupportedOperationsAsync(string id) => Pending?.Task ?? Task.FromResult(Caps() with { Identifier = id });
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest r) { Validations++; return Task.FromResult(new OperationValidation { Valid = true, Action = r.Action, Identifier = r.Identifier, ExpectedIdentity = r.ExpectedIdentity }); }
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest r) { Executions++; return Task.FromResult(new DiskOperationOutcome { Action = r.Action, Identifier = r.Identifier, Message = "Test-only outcome" }); }
        public Task<IReadOnlyList<Diskvio.Windows.Models.Disk>> ListDisksAsync() => throw new NotSupportedException();
    }
}
