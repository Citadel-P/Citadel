using Domain.Entities.Deployments;

namespace Domain.Contracts.Resources.Deployments;

public sealed record ApplyDeploymentCommand(
    string PlatformAddress,
    string ImageId,
    string Name,
    DeploymentSpec Spec
    );