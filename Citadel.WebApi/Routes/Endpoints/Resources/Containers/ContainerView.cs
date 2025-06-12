using Application.Permissions;
using Domain;
using Domain.Entities;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerView(
    Guid Id,
    string ContainerId,
    string Name,
    string Image,
    DateTimeOffset Created,
    ContainerStateStatus State,
    DateTimeOffset Updated,
    string? Stack,
    ContainerStatView? LastStats,
    IEnumerable<PortView>? Ports = null,
    PlatformView? Platform = null,
    EndpointMetadata? Metadata = null)
{
    internal static IEnumerable<ContainerView> Map(IEnumerable<Container> containersInfo)
        => containersInfo?.Select(Map) ?? [];

    internal static ContainerView Map(Container container)
    {
        return new (
            Id: container.Id,
            ContainerId: container.ContainerId,
            Name: container.Name,
            Image: container.Image,
            Created: DateTimeOffset.FromUnixTimeSeconds(container.Created),
            State: container.State,
            Updated: DateTimeOffset.FromUnixTimeSeconds(container.Updated),
            Stack: container.Stack,
            LastStats: container.Stats is not null && container.Stats.Count > 0 ? ContainerStatView.Map(container.Stats.First()) : null,
            Ports: container.Ports == null ? null : PortView.Map(container.Ports),
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