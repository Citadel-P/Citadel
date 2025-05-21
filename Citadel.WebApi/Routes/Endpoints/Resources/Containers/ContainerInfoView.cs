using Application.Permissions;
using Infrastructure;
using Infrastructure.Entities;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerInfoView(
    Guid Id,
    string ContainerId,
    string Name,
    string Image,
    DateTimeOffset Created,
     ContainerStateStatus State,
    string Status,
    string? Stack,
    ContainerStatView? LastStats,
    IEnumerable<PortView>? Ports = null,
    PlatformView? Platform = null,
    EndpointMetadata? Metadata = null)
{
    internal static IEnumerable<ContainerInfoView> Map(IEnumerable<ContainerInfo> containersInfo)
        => containersInfo?.Select(Map) ?? [];

    internal static ContainerInfoView Map(ContainerInfo container)
    {
        return new (
            Id: container.Id,
            ContainerId: container.ContainerId,
            Name: container.Name,
            Image: container.Image,
            Created: DateTimeOffset.FromUnixTimeSeconds(container.Created),
            State: container.State,
            Status: container.Status ?? "",
            Stack: container.Stack,
            LastStats: container.Stats is not null && container.Stats.Count > 0 ? ContainerStatView.Map(container.Stats.First()) : null,
            Ports: container.Ports == null ? null : PortView.Map(container.Ports),
            Platform: container.Platform == null ? null : PlatformView.Map(container.Platform));
    }

    internal static async Task<ContainerInfoView> Map(ContainerInfo container, IContainerPermissionService permissionService)
    {
        var permissions = await permissionService.GetContainerPermissions(container);
        return Map(container) with
        {
            Metadata = new (permissions.CanEdit, permissions.CanDelete)
        };
    }
};