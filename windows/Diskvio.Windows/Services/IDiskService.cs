using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public interface IDiskService
{
    Task<IReadOnlyList<Disk>> ListDisksAsync();
    Task<StorageInventory> InventoryAsync();
    Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request);
    Task<OperationValidation> ValidateAsync(DiskOperationRequest request);
    Task<OperationCapabilities> SupportedOperationsAsync(string identifier);
}
