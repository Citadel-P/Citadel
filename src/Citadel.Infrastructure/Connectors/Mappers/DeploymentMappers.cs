using Hosting.DockerClient.Models.Deployments;

namespace Infrastructure.Connectors.Mappers;

internal static class DeploymentMappers
{
    internal static Domain.Contracts.Resources.Deployments.ApplyDeploymentResult Map(this ApplyDeploymentResult dto)
        =>
        new Domain.Contracts.Resources.Deployments.ApplyDeploymentResult
        (
            ContainerId: dto.ContainerId,
            DeployedContainerState: dto.DeployedContainerState.Map()
        );

    internal static Domain.DeployedContainerState Map(this DeployedContainerState state)
         => state switch
         {
             DeployedContainerState.Running => Domain.DeployedContainerState.Running,
             DeployedContainerState.Exited => Domain.DeployedContainerState.Exited,
             DeployedContainerState.Timeout => Domain.DeployedContainerState.Timeout,
             _ => Domain.DeployedContainerState.Exited,
         };
}
