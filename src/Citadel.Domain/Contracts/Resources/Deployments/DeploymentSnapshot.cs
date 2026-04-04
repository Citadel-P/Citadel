using Domain.Entities.Deployments;

namespace Domain.Contracts.Resources.Deployments;

public sealed record DeploymentSnapshot(
        Guid Id,
        string Name,
        Guid PlatformId,
        string? Description = null,
        DeploymentSpec? Spec = null);

public static class DeploymentSnapshotExtensions
{
    public static DeploymentSnapshot ToSnapshot(this Deployment deployment, Guid? id = null)
        => new (
            Id: id ?? deployment.Id,
            Name: deployment.Name,
            PlatformId: deployment.PlatformId,
            Description: deployment.Description,
            Spec: deployment.Spec);
}

public sealed record DeploymentResultSnapshot(IEnumerable<string>? ContainerIds = null, string? Message = null);