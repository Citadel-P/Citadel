using Application.Permissions;
using Application.Features.Backups.Models;
using Application.Features.SwarmServices.Queries;
using Domain;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;
using WebApi.Routes.Endpoints.Resources.Swarm;

namespace WebApi.Routes.Endpoints.Resources.SwarmServices;

public sealed record SwarmServiceBackupSourcePreviewView(
    Guid SwarmServiceId,
    string SwarmServiceName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<SwarmServiceBackupVolumeView> Volumes,
    IReadOnlyList<string> Warnings)
{
    internal static SwarmServiceBackupSourcePreviewView Map(SwarmServiceBackupSourcePreviewResult result)
        => new(
            result.SwarmServiceId,
            result.SwarmServiceName,
            result.PlatformId,
            result.PlatformName,
            result.PlatformStatus,
            [.. result.Volumes.Select(SwarmServiceBackupVolumeView.Map)],
            result.Warnings);
}

public sealed record SwarmServiceBackupVolumeView(
    string Name,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared,
    bool HasBackupCoverage,
    string? DockerNodeId,
    string? NodeHostname)
{
    internal static SwarmServiceBackupVolumeView Map(StackBackupVolumePreviewItem item)
        => new(
            item.Name,
            item.Kind,
            item.IsExternal,
            item.IsShared,
            item.HasBackupCoverage,
            item.DockerNodeId,
            item.NodeHostname);
}

public sealed record ManagedSwarmServicesView(
    IEnumerable<ManagedSwarmServiceView> SwarmServices,
    ResourceCapabilities Capabilities)
{
    internal static async Task<ManagedSwarmServicesView> Map(
        ManagedSwarmServicesResult result,
        IPermissionEvaluator permissionEvaluator)
    {
        var list = result.Services;
        var resourcePermissions = await permissionEvaluator.EvaluateAsync(ResourceType.SwarmService);
        if (list.Count == 0)
            return new([], CapabilityMapper.ToResourceCapabilities(resourcePermissions));

        var permissions = await permissionEvaluator.EvaluateAsync(
            list.Select(static service => service.Id).ToArray(), ResourceType.SwarmService);
        var views = new ManagedSwarmServiceView[list.Count];
        for (var index = 0; index < list.Count; index++)
        {
            var service = list[index];
            permissions.TryGetValue(service.Id, out var metadata);
            result.TasksByServiceId.TryGetValue(service.Id, out var tasks);
            views[index] = ManagedSwarmServiceView.Map(service) with
            {
                Tasks = tasks?.Select(SwarmTaskView.Map).ToArray() ?? [],
                Capabilities = CapabilityMapper.ToSwarmServiceCapabilities(
                    metadata == default ? PermissionMetadata.Empty : metadata)
            };
        }
        return new(views, CapabilityMapper.ToResourceCapabilities(resourcePermissions));
    }
}

public sealed record ManagedSwarmServiceView(
    Guid Id,
    Guid PlatformId,
    string Name,
    string? Description,
    string DockerName,
    string? DockerServiceId,
    SwarmServiceSpec Spec,
    SwarmServiceHealth Health,
    SwarmServiceSynchronizationState SynchronizationState,
    ResourceControlState ControlState,
    AutoUpdateState AutoUpdateState,
    string? AppliedImageDigest,
    bool HasPendingDesiredChanges,
    bool HasRuntimeDrift,
    long RowVersion,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    string? PlatformName,
    PlatformStatus PlatformStatus,
    int? RunningTaskCount,
    int? DesiredTaskCount,
    string? UpdateState,
    string? UpdateMessage,
    SwarmServiceOperationView? CurrentOperation,
    IReadOnlyList<TagSummaryView> Tags,
    IReadOnlyList<SwarmTaskView>? Tasks = null,
    SwarmServiceCapabilities? Capabilities = null)
{
    internal static ManagedSwarmServiceView Map(SwarmService service) => new(
        service.Id,
        service.PlatformId,
        service.Name,
        service.Description,
        service.DockerName,
        service.DockerServiceId,
        service.Spec,
        service.Health,
        service.SynchronizationState,
        service.ControlState,
        service.AutoUpdateState,
        service.AppliedImageDigest,
        service.HasPendingChanges,
        service.HasRuntimeDrift,
        service.RowVersion,
        service.CreatedAt,
        service.UpdatedAt,
        service.Platform?.Name,
        service.Platform?.Status ?? PlatformStatus.Offline,
        service.Projection?.RunningTaskCount,
        service.Projection?.DesiredTaskCount,
        service.Projection?.UpdateState,
        service.Projection?.UpdateMessage,
        SwarmServiceOperationView.Map(service.CurrentOperation),
        [.. service.Tags.Select(TagSummaryView.Map)]);

    internal static async Task<ManagedSwarmServiceView> Map(
        SwarmService service,
        IPermissionEvaluator permissionEvaluator)
    {
        var permission = await permissionEvaluator.EvaluateAsync(service.Id, ResourceType.SwarmService);
        return Map(service) with { Capabilities = CapabilityMapper.ToSwarmServiceCapabilities(permission) };
    }
}

public sealed record SwarmServiceOperationView(
    Guid Id,
    SwarmServiceOperationKind Kind,
    SwarmServiceOperationState State,
    DateTime PreparedAt,
    DateTime? AttemptedAt,
    DateTime? CompletedAt,
    string? ResultCode,
    IReadOnlyList<string> Warnings,
    string? ResultMessage)
{
    internal static SwarmServiceOperationView? Map(SwarmServiceOperation? operation) => operation is null
        ? null
        : new(
            operation.Id,
            operation.Kind,
            operation.State,
            operation.PreparedAt,
            operation.AttemptedAt,
            operation.CompletedAt,
            operation.ResultCode,
            operation.Warnings ?? [],
            operation.ResultMessage);
}
