using Application.Features.Backups.Models;
using Application.Features.Tags.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Backups.Queries;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Read)]
public sealed record GetBackupRepositories : IQuery<Result<BackupRepositoryListResult>>;

[RequirePermission(ResourceType.BackupRepository, PermissionLevel.Read)]
public sealed record GetBackupRepository(Guid RepositoryId) : IQuery<Result<BackupRepository>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupPolicies(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<BackupPolicyListResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupPolicy(Guid PolicyId) : IQuery<Result<BackupPolicy>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupRuns(Guid? PolicyId = null, int Limit = 50) : IQuery<Result<BackupRunListResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupRun(Guid RunId) : IQuery<Result<BackupRun>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupRunLogs(Guid RunId) : IQuery<Result<BackupRunLogResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record GetBackupRestoreRuns(Guid? BackupRunId = null, Guid? PolicyId = null, int Limit = 50) : IQuery<Result<BackupRestoreRunListResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record GetBackupRestoreRun(Guid RestoreRunId) : IQuery<Result<BackupRestoreRun>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.Restore)]
public sealed record GetBackupRestoreRunLogs(Guid RestoreRunId) : IQuery<Result<BackupRestoreRunLogResult>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Read)]
public sealed record GetBackupCoverage(
    BackupCoverageResourceType CoverageResourceType,
    IReadOnlyList<BackupCoverageResourceKey> Resources)
    : IQuery<Result<BackupCoverageResult>>;

internal sealed class GetBackupRepositoriesHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRepositories, Result<BackupRepositoryListResult>>
{
    public async ValueTask<Result<BackupRepositoryListResult>> Handle(GetBackupRepositories query, CancellationToken cancellationToken)
    {
        var repositories = await unitOfWork.BackupRepositories.GetAllAsync(cancellationToken);
        return Result.Success(new BackupRepositoryListResult([.. repositories]));
    }
}

internal sealed class GetBackupRepositoryHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRepository, Result<BackupRepository>>
{
    public async ValueTask<Result<BackupRepository>> Handle(GetBackupRepository query, CancellationToken cancellationToken)
    {
        var repository = await unitOfWork.BackupRepositories.GetAsync(query.RepositoryId, cancellationToken);
        return repository is null
            ? Result.Failure<BackupRepository>(new NotFoundError("Backup repository not found."))
            : Result.Success(repository);
    }
}

internal sealed class GetBackupPoliciesHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetBackupPolicies, Result<BackupPolicyListResult>>
{
    public async ValueTask<Result<BackupPolicyListResult>> Handle(GetBackupPolicies query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success(new BackupPolicyListResult([]));

        var user = userContextAccessor.Current;
        var policies = user is not null && !user.IsAdmin
            ? await unitOfWork.BackupPolicies.GetAuthorizedAsync(
                user.UserId,
                ResourceType.BackupPolicy,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken,
                tagFilter.TagIds)
            : await unitOfWork.BackupPolicies.GetAllAsync(cancellationToken, tagFilter.TagIds);

        return Result.Success(new BackupPolicyListResult([.. policies]));
    }
}

internal sealed class GetBackupPolicyHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupPolicy, Result<BackupPolicy>>
{
    public async ValueTask<Result<BackupPolicy>> Handle(GetBackupPolicy query, CancellationToken cancellationToken)
    {
        var policy = await unitOfWork.BackupPolicies.GetAsync(query.PolicyId, cancellationToken);
        return policy is null
            ? Result.Failure<BackupPolicy>(new NotFoundError("Backup policy not found."))
            : Result.Success(policy);
    }
}

internal sealed class GetBackupRunsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRuns, Result<BackupRunListResult>>
{
    public async ValueTask<Result<BackupRunListResult>> Handle(GetBackupRuns query, CancellationToken cancellationToken)
    {
        var runs = query.PolicyId.HasValue
            ? await unitOfWork.BackupRuns.GetByPolicyAsync(query.PolicyId.Value, query.Limit, cancellationToken)
            : await unitOfWork.BackupRuns.GetPagedAsync(query.Limit, cancellationToken);

        return Result.Success(new BackupRunListResult([.. runs]));
    }
}

internal sealed class GetBackupRunHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRun, Result<BackupRun>>
{
    public async ValueTask<Result<BackupRun>> Handle(GetBackupRun query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BackupRuns.GetAsync(query.RunId, cancellationToken);
        return run is null
            ? Result.Failure<BackupRun>(new NotFoundError("Backup run not found."))
            : Result.Success(run);
    }
}

internal sealed class GetBackupRunLogsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRunLogs, Result<BackupRunLogResult>>
{
    public async ValueTask<Result<BackupRunLogResult>> Handle(GetBackupRunLogs query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BackupRuns.GetAsync(query.RunId, cancellationToken);
        if (run is null)
            return Result.Failure<BackupRunLogResult>(new NotFoundError("Backup run not found."));

        var logs = await unitOfWork.BackupRunLogs.GetByRunAsync(query.RunId, cancellationToken);
        return Result.Success(new BackupRunLogResult(query.RunId, logs));
    }
}

internal sealed class GetBackupRestoreRunsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRestoreRuns, Result<BackupRestoreRunListResult>>
{
    public async ValueTask<Result<BackupRestoreRunListResult>> Handle(GetBackupRestoreRuns query, CancellationToken cancellationToken)
    {
        var runs = query.BackupRunId.HasValue
            ? await unitOfWork.BackupRestoreRuns.GetByBackupRunAsync(query.BackupRunId.Value, query.Limit, cancellationToken)
            : query.PolicyId.HasValue
                ? await unitOfWork.BackupRestoreRuns.GetByPolicyAsync(query.PolicyId.Value, query.Limit, cancellationToken)
            : await unitOfWork.BackupRestoreRuns.GetPagedAsync(query.Limit, cancellationToken);

        return Result.Success(new BackupRestoreRunListResult([.. runs]));
    }
}

internal sealed class GetBackupRestoreRunHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRestoreRun, Result<BackupRestoreRun>>
{
    public async ValueTask<Result<BackupRestoreRun>> Handle(GetBackupRestoreRun query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BackupRestoreRuns.GetAsync(query.RestoreRunId, cancellationToken);
        return run is null
            ? Result.Failure<BackupRestoreRun>(new NotFoundError("Backup restore run not found."))
            : Result.Success(run);
    }
}

internal sealed class GetBackupRestoreRunLogsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBackupRestoreRunLogs, Result<BackupRestoreRunLogResult>>
{
    public async ValueTask<Result<BackupRestoreRunLogResult>> Handle(GetBackupRestoreRunLogs query, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.BackupRestoreRunLogs.GetByRunWithRunStateAsync(query.RestoreRunId, cancellationToken);
        if (!result.RunExists)
            return Result.Failure<BackupRestoreRunLogResult>(new NotFoundError("Backup restore run not found."));

        return Result.Success(new BackupRestoreRunLogResult(query.RestoreRunId, result.Logs));
    }
}

internal sealed class GetBackupCoverageHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetBackupCoverage, Result<BackupCoverageResult>>
{
    public async ValueTask<Result<BackupCoverageResult>> Handle(GetBackupCoverage query, CancellationToken cancellationToken)
    {
        if (query.Resources.Count == 0)
            return Result.Success(new BackupCoverageResult([]));

        if (query.CoverageResourceType != BackupCoverageResourceType.Volume)
            return Result.Failure<BackupCoverageResult>(new BadRequestError("Only volume backup coverage is currently supported."));

        var volumeKeys = new List<VolumeBackupCoverageKey>(query.Resources.Count);
        foreach (var resource in query.Resources)
        {
            if (!resource.PlatformId.HasValue || string.IsNullOrWhiteSpace(resource.Name))
                return Result.Failure<BackupCoverageResult>(new BadRequestError("Volume backup coverage requires platformId and name."));

            volumeKeys.Add(new VolumeBackupCoverageKey(resource.PlatformId.Value, resource.Name));
        }

        var user = userContextAccessor.Current;
        var coverage = await unitOfWork.BackupPolicies.GetVolumeCoverageAsync(
            volumeKeys,
            user is not null && !user.IsAdmin ? user.UserId : null,
            ResourceType.BackupPolicy,
            PermissionLevel.Read,
            SpecificPermission.None,
            cancellationToken);

        return Result.Success(new BackupCoverageResult(
            [.. coverage.Select(static item => new BackupCoverageItem(
                new BackupCoverageResourceKey(
                    PlatformId: item.Resource.PlatformId,
                    Name: item.Resource.VolumeName),
                item.Coverage))]));
    }
}
