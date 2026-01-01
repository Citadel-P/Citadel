using Domain.Contracts.Resources.Deployments;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IDeploymentConnector
{
    Task<Result<string>> ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken);
}
