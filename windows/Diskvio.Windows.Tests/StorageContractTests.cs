using System.Text.Json;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;

namespace Diskvio.Windows.Tests;

public sealed class StorageContractTests
{
    [Fact]
    public void PreservesVolumeActionsAndUnknownCapacity()
    {
        var inventory = DiskService.DecodeInventory("""
            {"status":"ok","inventory":{"disks":[{"number":2,"partition_scheme":"gpt","device":{"identifier":"PhysicalDrive2","name":"USB"},"partitions":[{"device":{"identifier":"partition","name":"Data"},"volumes":[{"device":{"identifier":"volume","name":"Données","actions":["unmount"],"identity_token":"token","available_bytes":20000000000},"label":"Données"}]}]}],"apfs_containers":[],"warnings":[]}}
            """);
        var volume = Assert.Single(Assert.Single(Assert.Single(inventory.Disks).Partitions).Volumes);
        Assert.Equal("Données", volume.Device.Name);
        Assert.Null(volume.Device.SizeBytes);
        Assert.Equal(20000000000ul, volume.Device.AvailableBytes);
        Assert.Equal(DiskAction.Unmount, Assert.Single(volume.Device.Actions));
    }

    [Fact]
    public void DecodesOperationQueriesAndOutcome()
    {
        var capabilities = DiskService.DecodeCapabilities("""{"status":"ok","capabilities":{"identifier":"volume","device_kind":"volume","identity_token":"token","actions":["mount"]}}""");
        Assert.Equal(DiskAction.Mount, Assert.Single(capabilities.Actions));
        Assert.True(DiskService.DecodeValidation("""{"status":"ok","validation":{"valid":true,"action":"mount","identifier":"volume","expected_identity":"token"}}""").Valid);
        Assert.Equal(DiskAction.Unmount, DiskService.DecodeOperation("""{"status":"ok","operation":{"action":"unmount","identifier":"volume","message":"Unmounted"}}""").Action);
        var request = JsonSerializer.Serialize(new DiskOperationRequest { Action = DiskAction.Mount, Identifier = "volume", ExpectedIdentity = "token" });
        Assert.Contains("\"action\":\"mount\"", request);
        Assert.Contains("\"expected_identity\":\"token\"", request);
    }

    [Theory]
    [InlineData("\"format\"")]
    [InlineData("0")]
    public void RejectsUnsupportedActions(string action) => Assert.Throws<DiskServiceException>(() => DiskService.DecodeCapabilities(
        $$$"""{"status":"ok","capabilities":{"identifier":"volume","device_kind":"volume","actions":[{{{action}}}]}}"""));

    [Fact]
    public void PreservesStructuredErrors()
    {
        var error = Assert.Throws<DiskServiceException>(() => DiskService.DecodeOperation("""{"status":"error","message":"Access denied","error":{"code":"permission_denied","message":"Access denied","platform_code":5}}"""));
        Assert.Equal("permission_denied", error.BackendError?.Code);
        Assert.Equal(5, error.BackendError?.PlatformCode);
    }

    [Theory]
    [InlineData("null")]
    [InlineData("{\"status\":\"ok\"}")]
    [InlineData("{\"status\":\"ok\",\"inventory\":{\"disks\":[null],\"apfs_containers\":[],\"warnings\":[]}}")]
    public void RejectsInvalidInventories(string json) => Assert.Throws<DiskServiceException>(() => DiskService.DecodeInventory(json));
}
