
using Application.Permissions;
using Application.Features.Backups.Models;
using Domain;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Deployments;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Identity;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Tags;
namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentsView(IEnumerable<DeploymentView> Deployments, ResourceCapabilities Capabilities)
{
    internal static async Task<DeploymentsView> Map(IEnumerable<Deployment> deployments, IPermissionEvaluator permissionEvaluator)
    {
        var list = deployments as Deployment[] ?? [.. deployments];

        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Deployment);

        if (list.Length == 0)
            return new DeploymentsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Deployment);

        var views = new DeploymentView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var deployment = list[i];

            var baseView = DeploymentView.Map(deployment);

            perms.TryGetValue(deployment.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToDeploymentCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new DeploymentsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
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
    IReadOnlyList<TagSummaryView> Tags = null!,
    LatestActivityView? LatestActivityView = null,
    DeploymentCapabilities? Capabilities = null
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
        Tags: [.. deployment.Tags.Select(TagSummaryView.Map)],
        LatestActivityView: deployment.LatestActivityEvent?.Map()
        );

    internal static async Task<DeploymentView> Map(Deployment deployment, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(deployment.Id, ResourceType.Deployment);
        return Map(deployment) with
        {
            Capabilities = CapabilityMapper.ToDeploymentCapabilities(permissions)
        };
    }
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

public sealed record DeploymentBackupSourcePreviewView(
    Guid DeploymentId,
    string DeploymentName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<DeploymentBackupVolumeView> Volumes,
    IReadOnlyList<string> Warnings)
{
    internal static DeploymentBackupSourcePreviewView Map(DeploymentBackupSourcePreviewResult result)
        => new(
            result.DeploymentId,
            result.DeploymentName,
            result.PlatformId,
            result.PlatformName,
            result.PlatformStatus,
            [.. result.Volumes.Select(DeploymentBackupVolumeView.Map)],
            result.Warnings);
}

public sealed record DeploymentBackupVolumeView(
    string Name,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared,
    bool HasBackupCoverage,
    string? DockerNodeId,
    string? NodeHostname)
{
    internal static DeploymentBackupVolumeView Map(StackBackupVolumePreviewItem item)
        => new(item.Name, item.Kind, item.IsExternal, item.IsShared, item.HasBackupCoverage, item.DockerNodeId, item.NodeHostname);
}
