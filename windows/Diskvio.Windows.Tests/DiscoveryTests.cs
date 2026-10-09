using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;

namespace Diskvio.Windows.Tests;

public sealed class DiscoveryTests
{
    private const string DiskJson = """
        {"number":7,"name":"Disque été 磁盘","size_bytes":18446744073709551615,"bus_type":"USB","partition_style":"GPT"}
        """;

    [Fact]
    public void DecodesExactRustTypesAndUnicode()
    {
        var disk = Assert.Single(DiskService.DecodeResponse($$"""{"status":"ok","disks":[{{DiskJson}}]}"""));
        Assert.Equal(7u, disk.Number);
        Assert.Equal(ulong.MaxValue, disk.SizeBytes);
        Assert.Equal("Disque été 磁盘", disk.Name);
        Assert.Equal(@"\\.\PhysicalDrive7", disk.Identifier);
        Assert.Equal("USB", disk.Connection);
        Assert.Equal("GPT", disk.Scheme);
    }

    [Fact]
    public void EmptyInventoryIsValid() => Assert.Empty(DiskService.DecodeResponse("""{"status":"ok","disks":[]} """));

    [Theory]
    [InlineData("null")]
    [InlineData("{}")]
    [InlineData("not JSON")]
    [InlineData("{\"status\":\"unknown\",\"disks\":[]}")]
    [InlineData("{\"status\":\"ok\"}")]
    [InlineData("{\"status\":\"ok\",\"disks\":[null]}")]
    [InlineData("{\"status\":\"ok\",\"disks\":[{\"number\":0}]}")]
    [InlineData("{\"status\":\"ok\",\"disks\":[{\"number\":-1,\"name\":\"x\",\"size_bytes\":1,\"bus_type\":\"USB\",\"partition_style\":\"GPT\"}]}")]
    [InlineData("{\"status\":\"ok\",\"disks\":[{\"number\":0,\"name\":null,\"size_bytes\":1,\"bus_type\":\"USB\",\"partition_style\":\"GPT\"}]}")]
    public void RejectsInvalidContracts(string json) => Assert.Throws<DiskServiceException>(() => DiskService.DecodeResponse(json));

    [Fact]
    public void RejectsDuplicateIdentifiers() => Assert.Throws<DiskServiceException>(
        () => DiskService.DecodeResponse($$"""{"status":"ok","disks":[{{DiskJson}},{{DiskJson}}]}"""));

    [Fact]
    public void PreservesBackendErrors()
    {
        var error = Assert.Throws<DiskServiceException>(() => DiskService.DecodeResponse("""{"status":"error","message":"Get-Disk failed"}"""));
        Assert.Equal("Get-Disk failed", error.Message);
    }

    [Fact]
    public async Task RefreshPreservesSelectionAndClearsStaleResultsOnFailure()
    {
        var service = new ControlledService();
        var viewModel = new DevicesViewModel(service);
        var first = MakeDisk(0);
        var second = MakeDisk(1);
        service.Result = [first, second];
        await viewModel.RefreshCommand.ExecuteAsync(null);
        viewModel.SelectedDisk = second;
        service.Result = [second with { SizeBytes = 2000 }, first];
        await viewModel.RefreshCommand.ExecuteAsync(null);
        Assert.Equal(1u, viewModel.SelectedDisk?.Number);
        Assert.Equal(2000ul, viewModel.SelectedDisk?.SizeBytes);
        Assert.False(viewModel.HasError);

        service.Result = [first];
        await viewModel.RefreshCommand.ExecuteAsync(null);
        Assert.Equal(0u, viewModel.SelectedDisk?.Number);
        service.Error = new DiskServiceException("Device discovery failed");
        await viewModel.RefreshCommand.ExecuteAsync(null);
        Assert.True(viewModel.HasError);
        Assert.False(viewModel.IsEmpty);
        Assert.False(viewModel.IsLoading);
        Assert.Empty(viewModel.Disks);
        Assert.Null(viewModel.SelectedDisk);

        service.Error = null;
        service.Result = [];
        await viewModel.RefreshCommand.ExecuteAsync(null);
        Assert.True(viewModel.IsEmpty);
        Assert.False(viewModel.HasError);
    }

    [Fact]
    public async Task LoadingDisablesConcurrentRefresh()
    {
        var service = new ControlledService { Pending = new(TaskCreationOptions.RunContinuationsAsynchronously) };
        var viewModel = new DevicesViewModel(service);
        var refresh = viewModel.RefreshCommand.ExecuteAsync(null);
        Assert.True(viewModel.IsLoading);
        Assert.False(viewModel.CanInspect);
        Assert.False(viewModel.RefreshCommand.CanExecute(null));
        service.Pending.SetResult([]);
        await refresh;
        Assert.False(viewModel.IsLoading);
        Assert.True(viewModel.CanInspect);
        Assert.True(viewModel.IsEmpty);
    }

    [Fact]
    [Trait("Category", "NativeIntegration")]
    public async Task RealRustDllDiscoversPhysicalDisksRepeatedly()
    {
        Assert.True(File.Exists(Path.Combine(AppContext.BaseDirectory, "diskvio_ffi.dll")));
        // Runs actual Rust -> Windows PowerShell -> Get-Disk twice and exercises
        // SafeHandle disposal. A backend error fails this integration test.
        var service = new DiskService();
        for (var attempt = 0; attempt < 2; attempt++)
        {
            var disks = await service.ListDisksAsync();
            Assert.Equal(disks.Count, disks.Select(disk => disk.Number).Distinct().Count());
            foreach (var disk in disks) Assert.NotNull(disk.Name);
        }
        NativeMethods.StringFree(IntPtr.Zero);
    }

    private static Disk MakeDisk(uint number) => new()
    {
        Number = number, Name = $"Test fixture {number}", SizeBytes = 1000, BusType = "USB", PartitionStyle = "GPT"
    };

    // Test-only fixtures. The production app always constructs DiskService.
    private sealed class ControlledService : IDiskService
    {
        public IReadOnlyList<Disk> Result { get; set; } = [];
        public Exception? Error { get; set; }
        public TaskCompletionSource<IReadOnlyList<Disk>>? Pending { get; init; }
        public Task<StorageInventory> InventoryAsync() => throw new NotSupportedException();
        public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request) => throw new NotSupportedException();
        public Task<OperationValidation> ValidateAsync(DiskOperationRequest request) => throw new NotSupportedException();
        public Task<OperationCapabilities> SupportedOperationsAsync(string identifier) => throw new NotSupportedException();
        public Task<IReadOnlyList<Disk>> ListDisksAsync() => Pending?.Task
            ?? (Error is null ? Task.FromResult(Result) : Task.FromException<IReadOnlyList<Disk>>(Error));
    }
}
