using Domain.Contracts.Resources.Deployments;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IDeploymentConnector
{
    Task<Result<ApplyDeploymentResult>> ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken);
}
