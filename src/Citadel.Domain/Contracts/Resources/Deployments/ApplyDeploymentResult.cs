namespace Domain.Contracts.Resources.Deployments;

public sealed record ApplyDeploymentResult(string ContainerId, DeployedContainerState DeployedContainerState);
