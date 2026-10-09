using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public interface IDiskService
{
    Task<IReadOnlyList<Disk>> ListDisksAsync();
}
