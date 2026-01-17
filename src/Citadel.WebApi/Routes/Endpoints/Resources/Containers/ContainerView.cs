using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerView(
    Guid Id,
    Guid PlatformId, 
    string ContainerId,
    string Name,
    string DockerImageId,
    long Created,
    ContainerStateStatus State,
    long Updated,
    string? Stack,
    ContainerStatView? LastStats,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    Guid? DeploymentId,
    PlatformView? Platform = null,
    ImageView? ImageView = null,
    DeploymentView? DeploymentView = null,
    EndpointMetadata? Metadata = null)
{
    internal static IEnumerable<ContainerView> Map(IEnumerable<Container> containersInfo)
        => containersInfo?.Select(Map).ToList() ?? [];

    internal static ContainerView Map(Container container)
    {
        string GetImageId()
        {
            if (string.IsNullOrEmpty(container.DockerImageId)) return string.Empty;

            ReadOnlySpan<char> span = container.DockerImageId.AsSpan();
            int idx = container.DockerImageId.IndexOf(':');

            return idx >= 0 ? span[(idx + 1)..].ToString() : container.DockerImageId;
        }

        return new (
            Id: container.Id,
            PlatformId: container.PlatformId,
            DeploymentId: container.DeploymentId,
            ContainerId: container.DockerContainerId,
            Name: container.Name,
            DockerImageId: GetImageId(),
            Created: container.Created,
            State: container.State,
            Updated: container.Updated,
            Stack: container.Stack,
            LastStats: container.Stats is not null && container.Stats.Count > 0 ? ContainerStatView.Map(container.Stats.First()) : null,
            Ports: container.Ports,
            ImageView: container.Image is not null ? ImagesView.Map(container.Image) : null,
            DeploymentView: container.Deployment is not null ? DeploymentView.Map(container.Deployment) : null,
            Platform: container.Platform == null ? null : PlatformView.Map(container.Platform));
    }

    internal static async Task<ContainerView> Map(Container container, IContainerPermissionService permissionService)
    {
        var permissions = await permissionService.GetContainerPermissions(container);
        return Map(container) with
        {
            Metadata = new (permissions.CanEdit, permissions.CanDelete)
        };
    }
};

public sealed record ContainerInfoView(
    string Name,
    string ContainerId,
    Guid PlatformId,
    string StartedAt,
    string FinishedAt,
    string PlatformName,
    IList<string> Volumes,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    IDictionary<string, string> Networks,
    ContainerStateStatus State,
    ImageView? ImageView,
    DeploymentView? DeploymentView = null)
{
    internal static ContainerInfoView Map(ContainerInfo container) => new(
        Name: container.Name,
        ContainerId: container.ContainerId,
        PlatformId: container.PlatformId,
        StartedAt: container.StartedAt,
        FinishedAt: container.FinishedAt,
        PlatformName: container.PlatformName,
        Volumes: container.Volumes,
        Ports: container.Ports,
        Networks: container.Networks,
        State: container.State,
        ImageView: container.Image is not null ? ImagesView.Map(container.Image) : null,
        DeploymentView: container.Deployment is not null ? DeploymentView.Map(container.Deployment) : null
        );
}
