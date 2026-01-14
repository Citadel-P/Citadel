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

    internal static Domain.Contracts.Resources.Deployments.ApplyDeploymentResult Map(this Citadel.Deployments.V1.ApplyDeploymentResponse dto)
        => new (ContainerId: dto.ContainerId, DeployedContainerState: dto.DeployedContainerState.Map());

    internal static Domain.DeployedContainerState Map(this DeployedContainerState state)
         => state switch
         {
             DeployedContainerState.Running => Domain.DeployedContainerState.Running,
             DeployedContainerState.Exited => Domain.DeployedContainerState.Exited,
             DeployedContainerState.Timeout => Domain.DeployedContainerState.Timeout,
             _ => Domain.DeployedContainerState.Exited,
         };

    internal static Domain.DeployedContainerState Map(this Citadel.Deployments.V1.DeployedContainerState state)
         => state switch
         {
             Citadel.Deployments.V1.DeployedContainerState.Running => Domain.DeployedContainerState.Running,
             Citadel.Deployments.V1.DeployedContainerState.Exited => Domain.DeployedContainerState.Exited,
             Citadel.Deployments.V1.DeployedContainerState.Timeout => Domain.DeployedContainerState.Timeout,
             _ => Domain.DeployedContainerState.Exited,
         };

    internal static Citadel.Deployments.V1.ContainerRestartPolicy MapToAgent(this Domain.ContainerRestartPolicy state)
         => state switch
         {
             Domain.ContainerRestartPolicy.OnFailure => Citadel.Deployments.V1.ContainerRestartPolicy.OnFailure,
             Domain.ContainerRestartPolicy.UnlessStopped => Citadel.Deployments.V1.ContainerRestartPolicy.UnlessStopped,
             Domain.ContainerRestartPolicy.Always => Citadel.Deployments.V1.ContainerRestartPolicy.Always,
             Domain.ContainerRestartPolicy.No => Citadel.Deployments.V1.ContainerRestartPolicy.No,
             _ => Citadel.Deployments.V1.ContainerRestartPolicy.No,
         };

    internal static Citadel.Deployments.V1.StopSignal MapToAgent(this Domain.StopSignal? signal)
         => signal switch
         {
             Domain.StopSignal.SIGTERM => Citadel.Deployments.V1.StopSignal.Sigterm,
             Domain.StopSignal.SIGINT => Citadel.Deployments.V1.StopSignal.Sigint,
             Domain.StopSignal.SIGKILL => Citadel.Deployments.V1.StopSignal.Sigkill,
             _ => Citadel.Deployments.V1.StopSignal.Sigterm,
         };

}
