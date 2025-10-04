using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerView(
    Guid Id,
    Guid PlatformId,
    string ContainerId,
    string Name,
    string ImageId,
    long Created,
    ContainerStateStatus State,
    long Updated,
    string? Stack,
    ContainerStatView? LastStats,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    PlatformView? Platform = null,
    ImageView? ImageView = null,
    EndpointMetadata? Metadata = null)
{
    internal static IEnumerable<ContainerView> Map(IEnumerable<Container> containersInfo)
        => containersInfo?.Select(Map).ToList() ?? [];

    internal static ContainerView Map(Container container)
    {
        string GetImageId()
        {
            if (string.IsNullOrEmpty(container.ImageId)) return string.Empty;

            ReadOnlySpan<char> span = container.ImageId.AsSpan();
            int idx = container.ImageId.IndexOf(':');

            return idx >= 0 ? span[(idx + 1)..].ToString() : container.ImageId;
        }

        return new (
            Id: container.Id,
            PlatformId: container.PlatformId,
            ContainerId: container.ContainerId,
            Name: container.Name,
            ImageId: GetImageId(), // container.ImageId,
            Created: container.Created,
            State: container.State,
            Updated: container.Updated,
            Stack: container.Stack,
            LastStats: container.Stats is not null && container.Stats.Count > 0 ? ContainerStatView.Map(container.Stats.First()) : null,
            Ports: container.Ports,
            ImageView: container.Image is not null ? ImagesView.Map(container.Image) : null,
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
    ImageView? ImageView)
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
        ImageView: container.Image is not null ? ImagesView.Map(container.Image) : null
        );
}
