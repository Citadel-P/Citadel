
using Domain;
using Domain.Entities.Deployments;
using WebApi.Routes.Endpoints.Resources.Activities;
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
    ResourceControlState ControlState,
    AutoUpdateState AutoUpdateState,
    DeploymentSpec Spec,
    PlatformStatus PlatformStatus,
    string? PlatformName = null,
    string? ImageName = null,
    Guid? ImageId = null,
    Guid? ContainerId = null,
    string? DockerContainerId = null,
    string? DockerImageId = null,
    LatestActivityView? LatestActivityView = null
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
        Spec: deployment.Spec,
        ControlState: deployment.ControlState,
        AutoUpdateState: deployment.AutoUpdateState ?? new AutoUpdateState(LastCheckedAt : DateTime.MinValue, Status: AutoUpdateStatus.Unknown),
        PlatformName: deployment.Platform?.Name,
        PlatformStatus: deployment.Platform?.Status ?? PlatformStatus.Offline,
        ImageName: deployment.Image?.Name,
        ImageId: deployment.Image?.Id,
        ContainerId: deployment.Container?.Id,
        DockerContainerId: deployment.Container?.DockerContainerId,
        DockerImageId: deployment.Container?.DockerImageId ?? deployment.Image?.DockerImageId,
        LatestActivityView: deployment.LatestActivityEvent?.Map()
        );
}

public sealed record DeploymentConfigView(
    Guid Id,
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec
    )
{
    internal static DeploymentConfigView Map(Deployment deployment) => new(
        Id: deployment.Id,
        Name: deployment.Name,
        PlatformId: deployment.PlatformId,
        Description: deployment.Description,
        Spec: deployment.Spec
        );
}