using Application.Features.Backups.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Queries;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, ResourceIdProperty = nameof(GetSwarmServiceBackupSourcePreview.SwarmServiceId))]
public sealed record GetSwarmServiceBackupSourcePreview(Guid SwarmServiceId)
    : IQuery<Result<SwarmServiceBackupSourcePreviewResult>>;

internal sealed class GetSwarmServiceBackupSourcePreviewHandler(
    ISwarmServiceBackupVolumeResolver resolver,
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetSwarmServiceBackupSourcePreview, Result<SwarmServiceBackupSourcePreviewResult>>
{
    public async ValueTask<Result<SwarmServiceBackupSourcePreviewResult>> Handle(
        GetSwarmServiceBackupSourcePreview query,
        CancellationToken cancellationToken)
    {
        var resolution = await resolver.ResolveAsync(query.SwarmServiceId, cancellationToken);
        if (!resolution.IsSuccess(out var resolved, out var error))
            return Result.Failure<SwarmServiceBackupSourcePreviewResult>(error);

        var keys = resolved.Volumes
            .Select(static volume => new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName, volume.DockerNodeId))
            .ToArray();
        var user = userContextAccessor.Current;
        IReadOnlyList<VolumeBackupCoverage> coverageRows = keys.Length == 0
            ? []
            : await unitOfWork.BackupPolicies.GetVolumeCoverageAsync(
                keys,
                user is not null && !user.IsAdmin ? user.ActorId : null,
                ResourceType.BackupPolicy,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken);
        var coverage = coverageRows.ToDictionary(static row => row.Resource, static row => row.Coverage);

        var volumes = resolved.Volumes.Select(volume =>
        {
            coverage.TryGetValue(
                new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName, volume.DockerNodeId),
                out var item);
            return new StackBackupVolumePreviewItem(
                volume.VolumeName,
                volume.Kind,
                volume.IsExternal,
                volume.IsShared,
                item?.Status is BackupCoverageStatus.Protected or BackupCoverageStatus.Warning,
                volume.DockerNodeId,
                volume.NodeHostname);
        }).ToArray();

        return Result.Success(new SwarmServiceBackupSourcePreviewResult(
            resolved.SwarmServiceId,
            resolved.SwarmServiceName,
            resolved.PlatformId,
            resolved.PlatformName,
            resolved.PlatformStatus,
            volumes,
            resolved.Warnings));
    }
}
