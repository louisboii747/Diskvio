using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public interface IPartitionPlanningService
{
    Task<PartitionPlanningResult> PlanPartitionAsync(PartitionPlanningRequest request);
}
