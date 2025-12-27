
using Domain;
using Domain.Entities;
namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentsView(IEnumerable<DeploymentView> Deployments)
{
    internal static DeploymentsView Map(IEnumerable<Deployment> deployments) => new(
        Deployments: deployments.Select(DeploymentView.Map)
        );
}

public sealed record DeploymentView(
    Guid Id,
    string Name,
    string? Description,
    Guid PlatformId,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    DeploymentStatus Status,
    DeploymentSpec Spec
    )
{
    internal static DeploymentView Map(Deployment deployment) => new(
        Id: deployment.Id,
        Name: deployment.Name,
        Description: deployment.Description,
        PlatformId: deployment.PlatformId,
        CreatedAt: deployment.CreatedAt,
        CreatedByActorId: deployment.CreatedByActorId,
        Status: deployment.Status,
        Spec: deployment.Spec
        );
}
