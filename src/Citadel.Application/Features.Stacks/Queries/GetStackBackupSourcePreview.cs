using Application.Features.Backups.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, ResourceIdProperty = nameof(GetStackBackupSourcePreview.StackId))]
public sealed record GetStackBackupSourcePreview(Guid StackId)
    : IQuery<Result<StackBackupSourcePreviewResult>>;

internal sealed class GetStackBackupSourcePreviewHandler(
    IStackBackupVolumeResolver resolver,
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetStackBackupSourcePreview, Result<StackBackupSourcePreviewResult>>
{
    public async ValueTask<Result<StackBackupSourcePreviewResult>> Handle(
        GetStackBackupSourcePreview query,
        CancellationToken cancellationToken)
    {
        var resolution = await resolver.ResolveAsync(query.StackId, cancellationToken);
        if (!resolution.IsSuccess(out var resolved, out var error))
            return Result.Failure<StackBackupSourcePreviewResult>(error);

        var coverage = await GetCoverageAsync(resolved, cancellationToken);
        var volumes = resolved.Volumes
            .Select(volume =>
            {
                coverage.TryGetValue(new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName, volume.DockerNodeId), out var item);
                return new StackBackupVolumePreviewItem(
                    volume.VolumeName,
                    volume.Kind,
                    volume.IsExternal,
                    volume.IsShared,
                    item?.Status is BackupCoverageStatus.Protected or BackupCoverageStatus.Warning,
                    volume.DockerNodeId,
                    volume.NodeHostname);
            })
            .ToArray();

        return Result.Success(new StackBackupSourcePreviewResult(
            resolved.StackId,
            resolved.StackName,
            resolved.PlatformId,
            resolved.PlatformName,
            resolved.PlatformStatus,
            volumes,
            resolved.Warnings));
    }

    private async Task<IReadOnlyDictionary<VolumeBackupCoverageKey, BackupCoverageView>> GetCoverageAsync(
        StackBackupVolumeResolution resolved,
        CancellationToken cancellationToken)
    {
        if (resolved.Volumes.Count == 0)
            return new Dictionary<VolumeBackupCoverageKey, BackupCoverageView>();

        var keys = resolved.Volumes
            .Select(static volume => new VolumeBackupCoverageKey(volume.PlatformId, volume.VolumeName, volume.DockerNodeId))
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
