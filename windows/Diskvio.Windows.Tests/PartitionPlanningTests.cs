using System.Reflection;
using System.Text.Json;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;

namespace Diskvio.Windows.Tests;

public sealed class PartitionPlanningTests
{
    private static string Fixture()
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("partition_planning_contract.json")!;
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    [Fact]
    public void SharedRustContractDecodesExactGeometryAndProtection()
    {
        var result = DiskService.DecodePartitionPlanning(Fixture());
        Assert.True(result.PreviewOnly);
        Assert.False(result.ExecutionSupported);
        Assert.Equal(104857600ul, result.Discovery?.Geometry?.DiskSizeBytes);
        var region = Assert.Single(result.Discovery!.Regions);
        Assert.Equal(98566144ul, region.MaximumAllocatableBytes);
        Assert.Equal(1048576ul, region.RequiredAlignmentBytes);
        Assert.Equal(4194304ul, result.CreatePreview?.AllocatedSizeBytes);
        var partition = Assert.Single(result.Discovery.Partitions);
        Assert.Contains("permanent data loss", partition.DataLossWarning);
        Assert.False(partition.Partition.Safety.Encrypted);
        Assert.Single(partition.Partition.Volumes);
    }

    [Fact]
    public void TypedCreateAndDeleteRequestsCannotBecomeExecutableActions()
    {
        var result = DiskService.DecodePartitionPlanning(Fixture());
        var discovery = result.Discovery!;
        PartitionPlanningRequest request = new CreatePartitionPlanRequest
        {
            Identifier = discovery.Identifier, ExpectedIdentity = discovery.PhysicalIdentity!, ExpectedLayout = discovery.LayoutSnapshot!,
            TestDisk = new() { Identity = discovery.PhysicalIdentity! }, Region = discovery.Regions[0].Identity,
            RequestedSizeBytes = 9007199254740993ul, RequiredAlignmentBytes = 1048576, PartitionTable = "gpt",
            ProposedGptType = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7"
        };
        var json = JsonSerializer.SerializeToElement(request);
        Assert.Equal("create", json.GetProperty("mode").GetString());
        Assert.Equal(9007199254740993ul, json.GetProperty("requested_size_bytes").GetUInt64());
        Assert.Equal(discovery.LayoutSnapshot!.Token, json.GetProperty("expected_layout").GetProperty("token").GetString());
        Assert.False(json.TryGetProperty("action", out _));
        var partition = discovery.Partitions[0].Partition;
        request = new DeletePartitionPlanRequest
        {
            Identifier = discovery.Identifier, ExpectedIdentity = discovery.PhysicalIdentity!, ExpectedLayout = discovery.LayoutSnapshot!,
            TestDisk = new() { Identity = discovery.PhysicalIdentity! }, Partition = new() { PartitionId = partition.PartitionId!, Number = partition.Number!.Value,
                OffsetBytes = partition.OffsetBytes!.Value, SizeBytes = partition.SizeBytes!.Value, GptType = partition.GptType! }
        };
        Assert.Equal("delete", JsonSerializer.SerializeToElement(request).GetProperty("mode").GetString());
        request = new DiscoverPartitionLayoutRequest { Identifier = "PhysicalDrive2" };
        Assert.Equal(2, JsonSerializer.SerializeToElement(request).EnumerateObject().Count());
    }

    [Theory]
    [InlineData("preview_only", "false")]
    [InlineData("execution_supported", "true")]
    [InlineData("api_version", "2")]
    public void RejectsUnexpectedBackendExecutionContracts(string field, string value)
    {
        var json = System.Text.Json.Nodes.JsonNode.Parse(Fixture())!;
        json["planning"]![field] = System.Text.Json.Nodes.JsonNode.Parse(value);
        Assert.Throws<DiskServiceException>(() => DiskService.DecodePartitionPlanning(json.ToJsonString()));
    }

    [Fact]
    public void UnsupportedAndPermissionFailuresRemainStructured()
    {
        var result = DiskService.DecodePartitionPlanning("""{"status":"ok","planning":{"api_version":1,"preview_only":true,"execution_supported":false,"validation":{"valid":false,"issues":[{"code":"unsupported_platform","message":"Unavailable on macOS"}]}}}""");
        Assert.False(result.Validation.Valid);
        Assert.Equal("unsupported_platform", Assert.Single(result.Validation.Issues).Code);
        var error = Assert.Throws<DiskServiceException>(() => DiskService.DecodePartitionPlanning("""{"status":"error","error":{"code":"permission_denied","message":"Read access denied","platform_code":5}}"""));
        Assert.Equal(5, error.BackendError?.PlatformCode);
    }

    [Fact]
    public async Task EligibilityInspectionUsesOnlyPlanningAndInvalidatesOnSelectionChange()
    {
        var service = new Fake();
        var vm = new StorageWorkspaceViewModel(service);
        await vm.RefreshAsync();
        await vm.InspectPartitionPlanningAsync();
        Assert.Equal(1, service.PlanningCalls);
        Assert.Contains("Preview only", vm.PartitionPlanningSummary);
        Assert.Contains("Aligned capacity", vm.PartitionPlanningSummary);
        Assert.False(vm.IsBusy);
        service.Pending = new();
        var pending = vm.InspectPartitionPlanningAsync();
        Assert.True(vm.IsBusy);
        Assert.False(vm.RefreshCommand.CanExecute(null));
        vm.SelectedNode = null;
        service.Pending.SetResult(DiskService.DecodePartitionPlanning(Fixture()));
        await pending;
        Assert.DoesNotContain("Aligned capacity", vm.PartitionPlanningSummary);
    }

    private sealed class Fake : IDiskService, IPartitionPlanningService
    {
        public int PlanningCalls { get; private set; }
        public TaskCompletionSource<PartitionPlanningResult>? Pending { get; set; }
        public Task<PartitionPlanningResult> PlanPartitionAsync(PartitionPlanningRequest request)
        {
            PlanningCalls++;
            Assert.IsType<DiscoverPartitionLayoutRequest>(request);
            return Pending?.Task ?? Task.FromResult(DiskService.DecodePartitionPlanning(Fixture()));
        }
        public Task<StorageInventory> InventoryAsync() => Task.FromResult(new StorageInventory { Disks = [new() { Device = new() { Identifier = "PhysicalDrive2", Name = "Test" }, Number = 2, PartitionScheme = "gpt", Partitions = [] }], ApfsContainers = [], Warnings = [] });
        public Task<OperationCapabilities> SupportedOperationsAsync(string identifier) => Task.FromResult(new OperationCapabilities { Identifier = identifier, DeviceKind = "physical_disk", Actions = [] });
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request) => throw new InvalidOperationException("Planning must not execute volume operations");
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest request) => throw new InvalidOperationException("Planning has a separate validation contract");
        public Task<IReadOnlyList<Disk>> ListDisksAsync() => throw new NotSupportedException();
    }
}
