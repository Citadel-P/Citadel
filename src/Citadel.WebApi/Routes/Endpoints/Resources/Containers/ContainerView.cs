using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Identity;
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
    ResourceControlState ControlState,
    long Updated,
    string? Stack,
    bool IsSystem,
    ContainerSystemRole? SystemRole,
    bool HasCitadelOwnershipLabels,
    ContainerStatView? LastStats,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    Guid? DeploymentId,
    Guid? StackId,
    PlatformView? Platform = null,
    ImageView? ImageView = null,
    DeploymentView? DeploymentView = null,
    PlatformCapabilities? Capabilities = null)
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
            StackId: container.StackId,
            ContainerId: container.DockerContainerId,
            Name: container.Name,
            DockerImageId: GetImageId(),
            Created: container.Created,
            State: container.State,
            ControlState: container.ControlState,
            Updated: container.Updated,
            Stack: container.DockerStack,
            IsSystem: container.IsSystem,
            SystemRole: container.SystemRole,
            HasCitadelOwnershipLabels: container.HasCitadelOwnershipLabels,
            LastStats: container.Stats is not null && container.Stats.Count > 0 ? ContainerStatView.Map(container.Stats.First()) : null,
            Ports: container.Ports,
            ImageView: container.Image is not null ? ImagesView.Map(container.Image) : null,
            DeploymentView: container.Deployment is not null ? DeploymentView.Map(container.Deployment) : null,
            Platform: container.Platform == null ? null : PlatformView.Map(container.Platform));
    }

    internal static async Task<ContainerView> Map(Container container, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(container.PlatformId, ResourceType.Platform);
        return Map(container) with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
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
    DeploymentView? DeploymentView = null,
    PlatformCapabilities? Capabilities = null)
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

    internal static async Task<ContainerInfoView> Map(ContainerInfo container, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(container.PlatformId, ResourceType.Platform);
        return Map(container) with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
        };
    }
}

public sealed record ContainersDataView(IEnumerable<ContainerDataView> Containers)
{
    internal static ContainersDataView Map(IEnumerable<DockerContainer> containers) =>
        new(containers.Select(ContainerDataView.Map));

}

public sealed record ContainerDataView(
    string Name,
    string Image,
    string Id,
    string ImageId,
    ContainerStateStatus State,
    ResourceControlState ControlState,
    bool IsSystem,
    ContainerSystemRole? SystemRole,
    bool HasCitadelOwnershipLabels,
    long? Created = null,
    string? Stack = null,
    ContainerStatView? ContainerStat = null,
    IDictionary<string, IReadOnlyList<HostPortBinding>>? Ports = null,
    Guid? DeploymentId = null,
    Guid? StackId = null,
    PlatformCapabilities? Capabilities = null
    )
{
    internal static ContainerDataView Map(DockerContainer container) => new(
        Name: container.Name,
        Image: container.Image,
        Id: container.Id,
        ImageId: container.ImageId,
        State: container.State,
        Created: container.Created,
        Stack: container.Stack,
        IsSystem: container.IsSystem,
        SystemRole: container.SystemRole,
        HasCitadelOwnershipLabels: container.HasCitadelOwnershipLabels,
        ControlState: container.ControlState ?? ResourceControlState.Idle,
        ContainerStat: null,
        Ports: container.Ports,
        DeploymentId: container.DeploymentId,
        StackId: container.StackId
        );

    internal static async Task<ContainerDataView> Map((DockerContainer container, Guid platformId) tuple, IPermissionEvaluator permissionEvaluator)
    {
        var (container, platformId) = tuple;
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        return Map(container) with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
        };
    }
}
