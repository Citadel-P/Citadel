using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Hosting.DockerClient.Models.Deployments;
using Hosting.DockerClient.Services;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalDeploymentConnector(IDeploymentService deploymentService) : IDeploymentConnector
{
    public Task<Result<string>> ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken)
    {
        var command = new ApplyDeploymentSpec
        (
            ImageId: applyDeployment.ImageId,
            Name: applyDeployment.Name,
            MemoryLimit: (long) applyDeployment.Spec.ResourceSpec.MemoryLimit,
            CpuQuota: (long) applyDeployment.Spec.ResourceSpec?.CpuLimit,
            RestartPolicy: applyDeployment.Spec.LifeCycleSpec?.RestartPolicy.Map(),
            Labels: applyDeployment.Spec.Labels,
            Networks: applyDeployment.Spec.Networks,
            Command: applyDeployment.Spec.Command,
            EnvVars: applyDeployment.Spec.EnvVars,
            Ports: applyDeployment.Spec.Ports,
            Volumes: applyDeployment.Spec.Volumes,
            StopTimeout: applyDeployment.Spec.LifeCycleSpec?.StopTimeout,
            StopSignal: applyDeployment.Spec.LifeCycleSpec?.StopSignal?.ToString()
        );
        return deploymentService.ApplyDeploymentAsync(command, cancellationToken);
    }
}
