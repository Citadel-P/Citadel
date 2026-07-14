using Application.Features.Backups.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentBackupSourcePreview(Guid DeploymentId)
    : IQuery<Result<DeploymentBackupSourcePreviewResult>>;

internal sealed class GetDeploymentBackupSourcePreviewHandler(
    IDeploymentBackupVolumeResolver resolver,
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetDeploymentBackupSourcePreview, Result<DeploymentBackupSourcePreviewResult>>
{
    public async ValueTask<Result<DeploymentBackupSourcePreviewResult>> Handle(
        GetDeploymentBackupSourcePreview query,
        CancellationToken cancellationToken)
    {
        var resolution = await resolver.ResolveAsync(query.DeploymentId, cancellationToken);
        if (!resolution.IsSuccess(out var resolved, out var error))
            return Result.Failure<DeploymentBackupSourcePreviewResult>(error);

        var coverage = await GetCoverageAsync(resolved, cancellationToken);
        var volumes = resolved.Volumes
            .Select(volume =>
            {
                coverage.TryGetValue(new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName), out var item);
                return new StackBackupVolumePreviewItem(
                    volume.VolumeName,
                    volume.Kind,
                    volume.IsExternal,
                    volume.IsShared,
                    item?.Status is BackupCoverageStatus.Protected or BackupCoverageStatus.Warning);
            })
            .ToArray();

        return Result.Success(new DeploymentBackupSourcePreviewResult(
            resolved.DeploymentId,
            resolved.DeploymentName,
            resolved.PlatformId,
            resolved.PlatformName,
            resolved.PlatformStatus,
            volumes,
            resolved.Warnings));
    }

    private async Task<IReadOnlyDictionary<VolumeBackupCoverageKey, BackupCoverageView>> GetCoverageAsync(
        DeploymentBackupVolumeResolution resolved,
        CancellationToken cancellationToken)
    {
        if (resolved.Volumes.Count == 0)
            return new Dictionary<VolumeBackupCoverageKey, BackupCoverageView>();

        var keys = resolved.Volumes
            .Select(static volume => new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName))
            .ToArray();

        var user = userContextAccessor.Current;
        var rows = await unitOfWork.BackupPolicies.GetVolumeCoverageAsync(
            keys,
            user is not null && !user.IsAdmin ? user.UserId : null,
            ResourceType.BackupPolicy,
            PermissionLevel.Read,
            SpecificPermission.None,
            cancellationToken);

        return rows.ToDictionary(static row => row.Resource, static row => row.Coverage);
    }
}
