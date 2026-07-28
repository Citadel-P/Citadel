using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record CheckStackUpdates(Guid StackId) : ICommand<Result<Stack>>;

internal sealed class CheckStackUpdatesHandler(
    IUnitOfWork unitOfWork,
    IImageCheckBuilder imageCheckBuilder,
    IImageDigestScanner imageDigestScanner,
    ManualStackUpdateEvaluator stackUpdateEvaluator,
    IManualStackDeployedImageResolver deployedImageResolver,
    IUpdateCheckLeaseManager leaseManager,
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository,
    IUserContextAccessor userContext,
    IStackStreamManager stackStreamManager,
    INotificationQueue notificationQueue,
    TimeProvider timeProvider,
    ILogger<CheckStackUpdatesHandler> logger)
    : ICommandHandler<CheckStackUpdates, Result<Stack>>
{
    private static readonly HashSet<StackReleaseStatus> AllowedReleaseStatuses =
    [
        StackReleaseStatus.Healthy,
        StackReleaseStatus.Degraded,
        StackReleaseStatus.Stopped,
        StackReleaseStatus.Paused
    ];

    public async ValueTask<Result<Stack>> Handle(
        CheckStackUpdates command,
        CancellationToken cancellationToken)
    {
        if (!leaseManager.TryAcquire(ResourceType.Stack, command.StackId, out var lease))
        {
            return Result.Failure<Stack>(
                new ConflictError("An update check is already running for this resource."));
        }

        using (lease)
        {
            var stack = await unitOfWork.Stacks.GetAsync(command.StackId, cancellationToken);
            if (stack?.CurrentStackRelease is null)
            {
                return Result.Failure<Stack>(
                    new NotFoundError("The provided stack does not exist."));
            }

            if (stack.ControlState == ResourceControlState.Processing)
            {
                return Result.Failure<Stack>(
                    new ConflictError("The stack is currently processing another operation."));
            }

            if (!AllowedReleaseStatuses.Contains(stack.CurrentStackRelease.Status))
            {
                return Result.Failure<Stack>(
                    new ConflictError("The stack release state does not support update checks."));
            }

            return (stack.StackSource, stack.CurrentStackRelease.Spec) switch
            {
                (StackSource.WebEditor, ManualStack) =>
                    await CheckManualStackAsync(stack, cancellationToken),
                (StackSource.Git, GitStack gitStack) =>
                    await CheckGitStackAsync(stack, gitStack, cancellationToken),
                _ => Result.Failure<Stack>(
                    new BadRequestError("The stack source and current release configuration are inconsistent."))
            };
        }
    }

    private async Task<Result<Stack>> CheckManualStackAsync(
        Stack stack,
        CancellationToken cancellationToken)
    {
        var checksResult = imageCheckBuilder.BuildManualStackChecks(
            stack,
            ImageCheckMode.OnDemand);
        if (checksResult.IsFailure(out var checksError, out var checks))
        {
            return Result.Failure<Stack>(checksError);
        }

        var registryId = checks[0].Key.RegistryId;
        var registry = await unitOfWork.Registries.GetAsync(registryId, cancellationToken);
        if (registry is null)
        {
            return Result.Failure<Stack>(
                new BadRequestError("The stack's configured registry does not exist."));
        }

        var scanTasks = new List<(ImageKey Key, ImageScanTask Task)>();
        foreach (var check in checks.GroupBy(item => item.Key).Select(group => group.First()))
        {
            var taskResult = imageCheckBuilder.BuildScanTask(
                check.Key,
                stack.CurrentStackRelease!.PlatformId,
                registry);
            if (taskResult.IsFailure(out var taskError, out var scanTask))
            {
                return Result.Failure<Stack>(taskError);
            }

            scanTasks.Add((check.Key, scanTask));
        }

        var claimResult = await ClaimUpdateCheckAsync(stack, cancellationToken);
        if (claimResult.IsFailure(out var claimError, out var claim))
        {
            return Result.Failure<Stack>(claimError);
        }

        try
        {
            await stackStreamManager.SendStackInfo(stack);

            var deployedDigestsResult = await deployedImageResolver.ResolveAsync(
                stack,
                checks,
                cancellationToken);
            if (deployedDigestsResult.IsFailure(out var deployedDigestsError, out var deployedDigests))
            {
                return Result.Failure<Stack>(deployedDigestsError);
            }

            var digests = new Dictionary<ImageKey, string>();
            foreach (var scan in scanTasks)
            {
                var scanResult = await imageDigestScanner.ScanAsync(scan.Task, cancellationToken);
                if (scanResult.IsFailure(out var scanError, out var digest))
                {
                    return Result.Failure<Stack>(scanError);
                }

                digests[scan.Key] = digest;
            }

            var evaluation = stackUpdateEvaluator.Evaluate(
                stack.StackUpdateState,
                checks,
                digests,
                timeProvider.GetUtcNow().UtcDateTime,
                deployedDigests);
            return await PersistStackStateAsync(
                stack,
                evaluation.State,
                claim.RowVersion,
                cancellationToken);
        }
        finally
        {
            if (stack.ControlState == ResourceControlState.Processing)
            {
                await ReleaseUpdateCheckAsync(
                    stack,
                    claim,
                    CancellationToken.None);
            }
        }
    }

    private async Task<Result<Stack>> CheckGitStackAsync(
        Stack stack,
        GitStack gitStack,
        CancellationToken cancellationToken)
    {
        var release = stack.CurrentStackRelease!;
        var source = release.Source;
        if (source is null
            || source.SourceType != StackSource.Git
            || source.GitRepositoryId != gitStack.GitRepoId
            || !string.Equals(source.Branch, gitStack.Branch, StringComparison.Ordinal)
            || string.IsNullOrWhiteSpace(source.ResolvedCommitSha))
        {
            return Result.Failure<Stack>(
                new ConflictError("The stack has no applied Git commit to compare."));
        }

        var repository = await unitOfWork.GitRepositories.GetWithAccountAsync(
            gitStack.GitRepoId,
            cancellationToken);
        if (repository is null)
        {
            return Result.Failure<Stack>(
                new BadRequestError("The stack's configured Git repository does not exist."));
        }

        if (repository.GitAccountId is not null && repository.GitAccount is null)
        {
            return Result.Failure<Stack>(
                new BadRequestError("The Git repository's configured account does not exist."));
        }

        string remoteUrl;
        try
        {
            remoteUrl = repoCacheManager.GetRemoteUrl(repository, repository.GitAccount);
        }
        catch (InvalidOperationException)
        {
            return Result.Failure<Stack>(
                new BadRequestError("The Git repository authentication configuration is incomplete."));
        }

        var claimResult = await ClaimUpdateCheckAsync(stack, cancellationToken);
        if (claimResult.IsFailure(out var claimError, out var claim))
        {
            return Result.Failure<Stack>(claimError);
        }

        try
        {
            await stackStreamManager.SendStackInfo(stack);

            RecreateStackOnNewCommitState commitState;
            try
            {
                var connectionResult = await gitCliRepository.TestConnectionAsync(
                    remoteUrl,
                    repository.GitAccount,
                    cancellationToken);
                if (connectionResult.IsFailure(out var connectionError))
                {
                    logger.LogWarning(
                        "Git update connection check failed for repository {RepositoryId}: {Error}",
                        repository.Id,
                        connectionError.Message);
                    return GitRemoteFailure();
                }

                var sync = await repoCacheManager.SynchronizeAsync(
                    repository,
                    repository.GitAccount,
                    source.Branch,
                    RepoSyncOptions.WithoutHooks,
                    cancellationToken);
                if (sync.Success != true
                    || string.IsNullOrWhiteSpace(sync.Hash)
                    || string.IsNullOrWhiteSpace(sync.CachePath))
                {
                    logger.LogWarning(
                        "Hook-free Git update synchronization failed for repository {RepositoryId}: {Error}",
                        repository.Id,
                        sync.Error);
                    return GitRemoteFailure();
                }

                var currentCommit = source.ResolvedCommitSha;
                var remoteCommit = sync.Hash;
                var relevantUpdate = false;
                if (!string.Equals(currentCommit, remoteCommit, StringComparison.OrdinalIgnoreCase))
                {
                    var pathsResult = await gitCliRepository.GetChangedPathsAsync(
                        sync.CachePath,
                        currentCommit,
                        remoteCommit,
                        cancellationToken);
                    if (pathsResult.IsFailure(out var pathsError, out var changedPaths))
                    {
                        logger.LogWarning(
                            "Git update diff failed for repository {RepositoryId}: {Error}",
                            repository.Id,
                            pathsError.Message);
                        return GitRemoteFailure();
                    }

                    relevantUpdate = GitStackWatchPathMatcher.HasRelevantChanges(
                        gitStack,
                        source,
                        changedPaths);
                }

                commitState = new RecreateStackOnNewCommitState(
                    CurrentCommitSha: currentCommit,
                    RemoteCommitSha: relevantUpdate ? remoteCommit : null,
                    LastCheckedAt: timeProvider.GetUtcNow().UtcDateTime);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogWarning(
                    ex,
                    "Git update check failed unexpectedly for repository {RepositoryId}",
                    repository.Id);
                return GitRemoteFailure();
            }

            var imageState = (stack.StackUpdateState as GitStackUpdateState)?
                .RecreateStackOnNewImageState
                ?? new RecreateStackOnNewImageState([]);
            var nextState = new GitStackUpdateState(
                imageState,
                commitState);

            return await PersistStackStateAsync(
                stack,
                nextState,
                claim.RowVersion,
                cancellationToken);
        }
        finally
        {
            if (stack.ControlState == ResourceControlState.Processing)
            {
                await ReleaseUpdateCheckAsync(
                    stack,
                    claim,
                    CancellationToken.None);
            }
        }
    }

    private async Task<Result<Stack>> PersistStackStateAsync(
        Stack stack,
        StackUpdateState state,
        long operationRowVersion,
        CancellationToken cancellationToken)
    {
        var release = stack.CurrentStackRelease!;
        var affectedRows = await unitOfWork.Stacks.TryCompleteUpdateCheckAsync(
            stack.Id,
            state,
            operationRowVersion,
            stack.CurrentStackReleaseId,
            release.Status,
            release.Spec,
            release.Source,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        if (affectedRows == 0)
        {
            return Result.Failure<Stack>(
                new ConflictError(
                    "The stack changed while the update check was running. Run the check again."));
        }

        stack.SetStackUpdateState(state);
        stack.ReleaseUpdateCheckProcessing();
        await notificationQueue.EnqueueAsync(
            new StackNotificationWorkItem(stackStreamManager, stack),
            cancellationToken);
        return Result.Success(stack);
    }

    private async Task<Result<UpdateCheckClaim>> ClaimUpdateCheckAsync(
        Stack stack,
        CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;
        if (!stack.MarkUpdateCheckProcessing(actorId))
        {
            return Result.Failure<UpdateCheckClaim>(
                new ConflictError("The stack is currently processing another operation."));
        }

        try
        {
            var release = stack.CurrentStackRelease!;
            var affected = await unitOfWork.Stacks.UpdateProcessingAsync(
                stack.Id,
                release.Status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion,
                checkRowVersion: true,
                actorId,
                cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);
            if (!affected)
            {
                stack.ReleaseUpdateCheckProcessing();
                return Result.Failure<UpdateCheckClaim>(
                    new ConflictError("The stack changed before the update check could start."));
            }

            return Result.Success(new UpdateCheckClaim(
                stack.RowVersion + 1,
                stack.ControlStartedAt!.Value,
                actorId));
        }
        catch
        {
            stack.ReleaseUpdateCheckProcessing();
            throw;
        }
    }

    private async Task<bool> ReleaseUpdateCheckAsync(
        Stack stack,
        UpdateCheckClaim claim,
        CancellationToken cancellationToken)
    {
        var affectedRows = await unitOfWork.Stacks.TryReleaseUpdateCheckAsync(
            stack.Id,
            claim.StartedAt,
            claim.ActorId,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        if (affectedRows == 0)
        {
            return false;
        }

        stack.ReleaseUpdateCheckProcessing();
        await notificationQueue.EnqueueAsync(
            new StackNotificationWorkItem(stackStreamManager, stack),
            cancellationToken);
        return true;
    }

    private static Result<Stack> GitRemoteFailure()
        => Result.Failure<Stack>(
            new BadGatewayError(
                "The Git update check failed. Verify repository connectivity and credentials."));

    private sealed record UpdateCheckClaim(
        long RowVersion,
        long StartedAt,
        Guid ActorId);
}
