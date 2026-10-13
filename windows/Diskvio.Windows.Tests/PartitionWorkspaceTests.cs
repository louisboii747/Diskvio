using System.Reflection;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;

namespace Diskvio.Windows.Tests;

public sealed class PartitionWorkspaceTests
{
    [Theory]
    [InlineData("1", "MiB", 1_048_576ul)]
    [InlineData("1.5", "GiB", 1_610_612_736ul)]
    [InlineData("0.00000095367431640625", "GiB", 1024ul)]
    [InlineData("9007199254740993", "bytes", 9_007_199_254_740_993ul)]
    [InlineData("18446744073709551615", "bytes", ulong.MaxValue)]
    public void ExactSizeConversionPreservesBytes(string text, string unit, ulong expected)
    {
        Assert.True(PartitionPlanInput.TryBytes(text, unit, out var bytes));
        Assert.Equal(expected, bytes);
    }

    [Theory]
    [InlineData("")]
    [InlineData("-1")]
    [InlineData("1e3")]
    [InlineData("1,024")]
    [InlineData(" 4")]
    [InlineData("NaN")]
    [InlineData("1.0000001")]
    [InlineData("18446744073709551615")]
    public void InvalidAndOverflowingSizeIsNeverRounded(string input)
        => Assert.False(PartitionPlanInput.TryBytes(input, "MiB", out _));

    [Fact]
    public async Task CreateRequestUsesSelectedBackendRegionIdentitySnapshotAndDesignation()
    {
        var (vm, service) = await Ready();
        using (vm)
        {
            Assert.False(vm.IsTestDiskDesignated);
            vm.SelectRegion(service.Discovery.Regions[0]);
            vm.BeginCreatePreview();
            Assert.False(vm.CanRequestPartitionPreview);
            Assert.Throws<InvalidOperationException>(() => vm.BuildPartitionRequest());
            vm.DesignateTestDisk(service.Discovery.PhysicalIdentity! with { SerialNumber = "different" });
            Assert.False(vm.IsTestDiskDesignated);
            vm.DesignateTestDisk(service.Discovery.PhysicalIdentity!);
            vm.RequestedSize = "4";
            vm.RequestedStart = "5242880";
            var request = Assert.IsType<CreatePartitionPlanRequest>(vm.BuildPartitionRequest());
            Assert.Same(service.Discovery.LayoutSnapshot, request.ExpectedLayout);
            Assert.Same(service.Discovery.Regions[0].Identity, request.Region);
            Assert.Equal(request.ExpectedIdentity, request.TestDisk.Identity);
            Assert.Equal(4_194_304ul, request.RequestedSizeBytes);
            Assert.Equal(5_242_880ul, request.RequestedStartOffsetBytes);
            Assert.Equal(1_048_576ul, request.RequiredAlignmentBytes);
            Assert.Equal(PartitionPlanInput.BasicDataType, request.ProposedGptType);
            await vm.PreviewPartitionAsync();
            Assert.True(vm.HasValidatedPreview);
            Assert.Contains("Remaining geometric capacity", vm.PreviewDetails);
            Assert.NotNull(vm.PreviewLayout);
            Assert.Equal(service.Discovery.Geometry!.DiskSizeBytes, vm.PreviewLayout!.Segments.Aggregate(0ul, (sum, s) => sum + s.Length));
            Assert.Single(vm.Layout!.Segments, s => s.Region is not null);
            Assert.Contains(vm.PreviewLayout.Segments, s => s.Proposal == "Proposed basic data (preview)" && s.Length == request.RequestedSizeBytes);
            Assert.Equal(2, service.Requests.Count);
        }
    }

    [Fact]
    public async Task DeletionUsesPartitionIdentityAndProjectsOnlyTheSelectedPartition()
    {
        var (vm, service) = await Ready();
        using (vm)
        {
            vm.SelectPartition(vm.SelectedDisk!.Disk!.Partitions[0]);
            Assert.True(vm.CanBeginDeletePreview);
            vm.BeginDeletePreview();
            vm.DesignateTestDisk(service.Discovery.PhysicalIdentity!);
            var request = Assert.IsType<DeletePartitionPlanRequest>(vm.BuildPartitionRequest());
            Assert.Equal(PartitionPlanInput.Target(service.Discovery.Partitions[0].Partition), request.Partition);
            Assert.Contains("E:", vm.DeletionSummary);
            Assert.Contains("permanent data loss", vm.DeletionWarning);
            service.Response = service.Response with { CreatePreview = null, DeletePreview = service.Discovery.Partitions[0] };
            await vm.PreviewPartitionAsync();
            Assert.True(vm.HasValidatedPreview);
            Assert.Single(vm.Layout!.Segments, s => s.Partition is not null);
            Assert.DoesNotContain(vm.PreviewLayout!.Segments, s => s.Partition is not null);
            Assert.Contains(vm.PreviewLayout.Segments, s => s.Proposal == "Unallocated after deletion (preview)" && s.Length == request.Partition.SizeBytes);
            Assert.Single(vm.SelectedDisk.Disk.Partitions);
        }
    }

    [Fact]
    public async Task RegionAndPartitionSelectionUseTheExistingExplorerAndInvalidatePlans()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            await vm.PreviewPartitionAsync();
            Assert.True(vm.HasValidatedPreview);
            Assert.Equal(vm.SelectedNode, Assert.Single(vm.Children, n => n.Region is not null));
            Assert.Equal(PartitionPlanInput.RegionId(service.Discovery.Regions[0]), vm.SelectedMapId);
            var capabilityCalls = service.CapabilityCalls;
            await vm.QueryCapabilitiesAsync();
            Assert.Equal(capabilityCalls, service.CapabilityCalls);
            vm.SelectPartition(vm.SelectedDisk!.Disk!.Partitions[0]);
            Assert.False(vm.HasValidatedPreview);
            Assert.False(vm.IsPlanningEditor);
            Assert.Null(vm.PreviewLayout);
            Assert.NotNull(vm.PlanningDiscovery);
            Assert.True(vm.IsTestDiskDesignated);
            Assert.Equal(vm.SelectedNode, vm.Children[0]);
        }
    }

    [Fact]
    public async Task RefreshAndDeviceRemovalRevokeAllPlanningState()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            await vm.PreviewPartitionAsync();
            service.Inventory = service.Inventory with { Disks = [] };
            await vm.RefreshAsync();
            Assert.False(vm.HasValidatedPreview);
            Assert.False(vm.IsTestDiskDesignated);
            Assert.Null(vm.PlanningDiscovery);
            Assert.Null(vm.SelectedDisk);
            Assert.False(vm.CanRequestPartitionPreview);
        }
    }

    [Fact]
    public async Task LatePreviewAndBusyRequestsCannotReplaceNewSelection()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            service.Pending = new();
            var task = vm.PreviewPartitionAsync();
            Assert.True(vm.IsBusy);
            Assert.False(vm.CanRequestPartitionPreview);
            await vm.PreviewPartitionAsync();
            Assert.Equal(2, service.Requests.Count);
            vm.SelectDiskDetails();
            service.Pending.SetResult(service.Response);
            await task;
            Assert.False(vm.IsBusy);
            Assert.False(vm.HasValidatedPreview);
            Assert.False(vm.IsPlanningEditor);
        }
    }

    [Fact]
    public async Task RefreshDuringNativeCallDiscardsResponseAndDrainsDeferredInventory()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            service.Pending = new();
            var task = vm.PreviewPartitionAsync();
            await vm.RefreshAsync();
            Assert.Null(vm.PlanningDiscovery);
            service.Inventory = service.Inventory with { Disks = [] };
            service.Pending.SetResult(service.Response);
            await task;
            Assert.Equal(2, service.InventoryCalls);
            Assert.Null(vm.SelectedDisk);
            Assert.False(vm.HasValidatedPreview);
        }
    }

    [Theory]
    [InlineData("changed_partition_layout", "Stale identity or layout")]
    [InlineData("stale_device_identity", "Stale identity or layout")]
    [InlineData("device_disappeared", "Device unavailable")]
    [InlineData("protected_target", "Protected or unverified target")]
    [InlineData("unsupported_operation", "Unsupported planning operation")]
    [InlineData("discovery_failed", "Backend discovery failed")]
    [InlineData("insufficient_allocatable_space", "Preview rejected")]
    public async Task RejectionNeverRetainsAnEligiblePreview(string code, string status)
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            await vm.PreviewPartitionAsync();
            service.Response = service.Response with { Validation = Rejected(code) };
            await vm.PreviewPartitionAsync();
            Assert.False(vm.HasValidatedPreview);
            Assert.Null(vm.PartitionPreview);
            Assert.Equal(status, vm.PlanningStatus);
            Assert.Contains(code, vm.PartitionPlanningSummary);
        }
    }

    [Theory]
    [InlineData("protected_target")]
    [InlineData("unsupported_platform")]
    [InlineData("missing_metadata")]
    public async Task RejectedDiscoveryAndProtectedPartitionsCannotBeDesignated(string code)
    {
        var service = new Fake();
        var validation = Rejected(code);
        service.Response = service.Response with { Validation = validation, Discovery = service.Discovery with { Validation = validation,
            Regions = [service.Discovery.Regions[0] with { Validation = validation, MaximumAllocatableBytes = 0 }],
            Partitions = [service.Discovery.Partitions[0] with { Validation = validation, Eligible = false }] } };
        using var vm = new StorageWorkspaceViewModel(service);
        await vm.RefreshAsync(); await vm.InspectPartitionPlanningAsync();
        Assert.False(vm.CanDesignateTestDisk);
        vm.DesignateTestDisk(service.Discovery.PhysicalIdentity!);
        Assert.False(vm.IsTestDiskDesignated);
        vm.SelectRegion(vm.PlanningDiscovery!.Regions[0]);
        Assert.False(vm.CanBeginCreatePreview);
        vm.SelectPartition(vm.SelectedDisk!.Disk!.Partitions[0]);
        vm.BeginDeletePreview();
        Assert.True(vm.IsDeleteEditor);
        Assert.Contains("Protected", vm.DeletionSummary);
        Assert.False(vm.CanRequestPartitionPreview);
        await vm.PreviewPartitionAsync();
        Assert.Single(service.Requests);
    }

    [Fact]
    public async Task InvalidInputsDoNotDispatchAndInputChangesClearPreviousPreview()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            await vm.PreviewPartitionAsync();
            vm.RequestedSize = "invalid";
            Assert.False(vm.HasValidatedPreview);
            await vm.PreviewPartitionAsync();
            Assert.Equal(2, service.Requests.Count);
            Assert.Contains("exact size", vm.PlanningStatus);
            vm.RequestedSize = "4"; vm.RequestedStart = "-1";
            await vm.PreviewPartitionAsync();
            Assert.Contains("offset", vm.PlanningStatus);
            Assert.Equal(2, service.Requests.Count);
            vm.CancelPartitionPreview();
            Assert.False(vm.IsPlanningEditor);
            Assert.Null(vm.PreviewLayout);
        }
    }

    [Fact]
    public async Task DifferentDiskAndUnexpectedResponseIdentityRevokeDesignation()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            service.Response = service.Response with { Discovery = service.Discovery with { PhysicalIdentity = service.Discovery.PhysicalIdentity! with { SerialNumber = "replacement" } } };
            await vm.PreviewPartitionAsync();
            Assert.False(vm.HasValidatedPreview);
            Assert.False(vm.IsTestDiskDesignated);
            Assert.Null(vm.PlanningDiscovery);
            Assert.Contains("identity", vm.PartitionPlanningSummary);
            vm.SelectedDisk = new(service.Inventory.Disks[0].Device with { Identifier = "PhysicalDrive8" }, "Physical disk", service.Inventory.Disks[0]);
            Assert.Null(vm.PlanningDiscovery);
            Assert.False(vm.IsTestDiskDesignated);
        }
    }

    [Fact]
    public async Task CancelAndEditedInputsDiscardInFlightResults()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            service.Pending = new();
            var task = vm.PreviewPartitionAsync();
            vm.RequestedStart = "6291456";
            service.Pending.SetResult(service.Response);
            await task;
            Assert.False(vm.HasValidatedPreview);
            Assert.Contains("Inputs changed", vm.PlanningStatus);
            service.Pending = new();
            task = vm.PreviewPartitionAsync();
            vm.CancelPartitionPreview();
            service.Pending.SetResult(service.Response);
            await task;
            Assert.False(vm.HasValidatedPreview);
            Assert.False(vm.IsPlanningEditor);
            Assert.Contains("cancelled", vm.PlanningStatus);
        }
    }

    [Fact]
    public async Task PlanningDiscoveryMustAgreeWithTheExistingInventory()
    {
        var service = new Fake();
        service.Inventory = service.Inventory with { Disks = [service.Inventory.Disks[0] with { Partitions = [] }] };
        using var vm = new StorageWorkspaceViewModel(service);
        await vm.RefreshAsync();
        await vm.InspectPartitionPlanningAsync();
        Assert.Null(vm.PlanningDiscovery);
        Assert.Equal("Stale identity or layout", vm.PlanningStatus);
        Assert.DoesNotContain(vm.Children, node => node.Region is not null);
    }

    [Fact]
    public async Task DebouncedInputRequestsTheLatestExactValues()
    {
        var (vm, service) = await ReadyCreate();
        using (vm)
        {
            vm.RequestedSize = "2";
            vm.RequestedSize = "4";
            await Task.Delay(650);
            Assert.Equal(2, service.Requests.Count);
            Assert.Equal(4_194_304ul, Assert.IsType<CreatePartitionPlanRequest>(service.Requests[1]).RequestedSizeBytes);
            Assert.True(vm.HasValidatedPreview);
        }
    }

    private static PlanningValidation Rejected(string code) => new() { Valid = false, Issues = [new() { Code = code, Message = $"Backend restriction: {code}" }] };
    private static async Task<(StorageWorkspaceViewModel, Fake)> Ready()
    {
        var service = new Fake(); var vm = new StorageWorkspaceViewModel(service);
        await vm.RefreshAsync(); await vm.InspectPartitionPlanningAsync(); return (vm, service);
    }
    private static async Task<(StorageWorkspaceViewModel, Fake)> ReadyCreate()
    {
        var (vm, service) = await Ready();
        vm.SelectRegion(service.Discovery.Regions[0]); vm.BeginCreatePreview();
        vm.DesignateTestDisk(service.Discovery.PhysicalIdentity!); vm.RequestedSize = "4";
        return (vm, service);
    }

    private sealed class Fake : IDiskService, IPartitionPlanningService
    {
        public PartitionPlanningResult Response { get; set; }
        public DiskPlanningDiscovery Discovery { get; }
        public StorageInventory Inventory { get; set; }
        public List<PartitionPlanningRequest> Requests { get; } = [];
        public TaskCompletionSource<PartitionPlanningResult>? Pending { get; set; }
        public int InventoryCalls { get; private set; }
        public int CapabilityCalls { get; private set; }
        public Fake()
        {
            using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("partition_planning_contract.json")!;
            using var reader = new StreamReader(stream);
            Response = DiskService.DecodePartitionPlanning(reader.ReadToEnd());
            Discovery = Response.Discovery!;
            var partition = Discovery.Partitions[0].Partition;
            Inventory = new() { Disks = [new() { Number = 2, Device = new() { Identifier = Discovery.Identifier, Name = Discovery.Name, SizeBytes = Discovery.Geometry!.DiskSizeBytes },
                PartitionScheme = "gpt", DiskUuid = Discovery.Geometry.DiskGuid, Internal = false, ConnectionType = "USB",
                Partitions = [new() { Number = partition.Number, OffsetBytes = partition.OffsetBytes, GptType = partition.GptType, Device = new() { Identifier = "PhysicalDrive2Partition1", Name = "Test partition", PartitionUuid = partition.PartitionId, SizeBytes = partition.SizeBytes } }] }], ApfsContainers = [], Warnings = [] };
        }
        public Task<PartitionPlanningResult> PlanPartitionAsync(PartitionPlanningRequest request) { Requests.Add(request); return Pending?.Task ?? Task.FromResult(Response); }
        public Task<StorageInventory> InventoryAsync() { InventoryCalls++; return Task.FromResult(Inventory); }
        public Task<OperationCapabilities> SupportedOperationsAsync(string identifier) { CapabilityCalls++; return Task.FromResult(new OperationCapabilities { Identifier = identifier, DeviceKind = "physical_disk", Actions = [] }); }
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request) => throw new InvalidOperationException("Partition planning cannot execute.");
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest request) => throw new InvalidOperationException("Partition planning cannot dispatch volume validation.");
        public Task<IReadOnlyList<Disk>> ListDisksAsync() => throw new NotSupportedException();
    }
}
