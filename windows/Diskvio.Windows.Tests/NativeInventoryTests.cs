using Diskvio.Windows.Services;
namespace Diskvio.Windows.Tests;
public sealed class NativeInventoryTests
{
    [Fact] [Trait("Category", "NativeIntegration")]
    public async Task RealRustInventoryAndCapabilitiesRoundTripWithoutOperations()
    {
        var service = new DiskService();
        for (var i = 0; i < 2; i++)
        {
            var inventory = await service.InventoryAsync();
            Assert.NotEmpty(inventory.Disks);
            Assert.Equal(inventory.Disks.Count, inventory.Disks.Select(d => d.Device.Identifier).Distinct().Count());
            foreach (var disk in inventory.Disks)
            {
                var caps = await service.SupportedOperationsAsync(disk.Device.Identifier);
                Assert.Equal(disk.Device.Identifier, caps.Identifier);
                Assert.DoesNotContain(Models.DiskAction.Eject, caps.Actions);
                foreach (var partition in disk.Partitions)
                {
                    Assert.NotNull(partition.Role);
                    Assert.Equal(Models.PartitionPresentation.Name(partition), partition.Device.Name);
                    foreach (var volume in partition.Volumes)
                    {
                        var volumeCaps = await service.SupportedOperationsAsync(volume.Device.Identifier);
                        Assert.Equal("volume", volumeCaps.DeviceKind);
                        Assert.Equal(volume.Device.IdentityToken, volumeCaps.IdentityToken);
                        if (disk.Device.Safety.System == true || disk.Device.Safety.Boot == true || volume.Device.Safety.PageFile == true)
                            Assert.Empty(volumeCaps.Actions);
                        if (volumeCaps.Actions.Contains(Models.DiskAction.RenameVolume))
                        {
                            Assert.False(volume.Device.Safety.PageFile);
                            Assert.False(disk.Internal);
                            Assert.NotNull(volumeCaps.LabelMaxLength);
                        }
                    }
                }
            }
        }
    }
}
