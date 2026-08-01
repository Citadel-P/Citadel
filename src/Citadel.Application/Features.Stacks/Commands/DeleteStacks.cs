using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Text;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Execute)]
public sealed record DeleteStacks(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteStacksHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IStackStreamManager stackHub,
    IPlatformStreamManager platformHub,
    IStackStoragePathProvider stackStoragePathProvider,
    IUserContextAccessor userContext,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeleteStacksHandler> logger) : ICommandHandler<DeleteStacks, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(DeleteStacks command, CancellationToken cancellationToken)
    {
        var requestedIds = command.Ids.Distinct().Order().ToArray();
        var stacks = (await unitOfWork.Stacks.GetAllAsync(requestedIds, cancellationToken) ?? [])
            .OrderBy(stack => stack.Id)
            .ToArray();
        if (requestedIds.Length == 0 || stacks.Length != requestedIds.Length)
        {
            return Result.Failure(new NotFoundError("One or more stacks were not found."));
        }

        var claimResult = await ClaimStacksAsync(stacks, userContext.Current.ActorId, cancellationToken);
        if (claimResult.IsFailure(out var claimError, out var previousStatuses))
            return Result.Failure(claimError);

        // Once the optimistic claim is committed, finish or roll it back independently of
        // the HTTP request. Otherwise a disconnected client can strand a stack after Docker
        // has already accepted one of the deletes.
        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;

        var deletionCommitted = false;
        try
        {
            var plans = new List<StackRuntimeDeletePlan>();
            foreach (var stack in stacks)
            {
                var planResult = await PrepareRuntimeContainerDeletionAsync(
                    stack,
                    previousStatuses[stack.Id],
                    completionToken);
                if (planResult.IsFailure(out var planError, out var plan))
                {
                    await TryRollbackClaimsAsync(stacks, previousStatuses);
                    return Result.Failure(planError);
                }

                if (plan is not null)
                    plans.Add(plan);
            }

            // All ownership and connectivity checks complete before the first destructive
            // Docker call. A retry is safe if a later daemon delete fails because missing
            // containers produce an empty plan while the database stack still exists.
            foreach (var plan in plans)
            {
                var cleanupResult = await plan.Connector.DeleteAsync(plan.Command, completionToken);
                if (cleanupResult.IsFailure(out var cleanupError))
                {
                    await TryRollbackClaimsAsync(stacks, previousStatuses);
                    return Result.Failure(cleanupError);
                }
            }

            var platformIds = stacks
                .Select(stack => stack.CurrentStackRelease?.PlatformId)
                .Where(platformId => platformId.HasValue)
                .Select(platformId => platformId!.Value)
                .Distinct()
                .ToArray();

            var deleted = await unitOfWork.Stacks.RemoveRangeAsync(requestedIds, completionToken);
            if (deleted != stacks.Length)
            {
                await unitOfWork.RollbackAsync();
                await TryRollbackClaimsAsync(stacks, previousStatuses);
                return Result.Failure(new ConflictError("The stack set changed while deletion was in progress."));
            }

            var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatByIdsAsync(
                platformIds,
                completionToken);
            await unitOfWork.CommitAsync(completionToken);
            deletionCommitted = true;

            foreach (var stack in stacks)
                TryDeleteStackStorage(stack);

            using var postCommitCancellation = CancellationTokenSource.CreateLinkedTokenSource(
                applicationLifetime.ApplicationStopping,
                completionToken);
            postCommitCancellation.CancelAfter(TimeSpan.FromSeconds(5));
            foreach (var stack in stacks)
                await RunPostCommitStepAsync(
                    () => stackHub.SendStackInfo(stack, "delete"),
                    stack.Id,
                    "stack deletion notification",
                    postCommitCancellation.Token);

            foreach (var platform in platforms)
                await RunPostCommitStepAsync(
                    () => platformHub.PushPlatformUpdate(platform),
                    platform.Id,
                    "platform update notification",
                    postCommitCancellation.Token);

            return Result.Success();
        }
        catch
        {
            if (!deletionCommitted)
                await TryRollbackClaimsAsync(stacks, previousStatuses);
            throw;
        }
    }

    private async Task TryRollbackClaimsAsync(
        IReadOnlyCollection<Stack> stacks,
        IReadOnlyDictionary<Guid, StackReleaseStatus> previousStatuses)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await RollbackClaimsAsync(stacks, previousStatuses, rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back stack deletion claims");
        }
    }

    private async Task RunPostCommitStepAsync(
        Func<Task> action,
        Guid resourceId,
        string step,
        CancellationToken cancellationToken)
    {
        try
        {
            await action().WaitAsync(cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed {Step} for deleted resource {ResourceId}", step, resourceId);
        }
    }

    private async Task<Result<IReadOnlyDictionary<Guid, StackReleaseStatus>>> ClaimStacksAsync(
        IReadOnlyCollection<Stack> stacks,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        if (stacks.Any(stack =>
                stack.CurrentStackRelease is null ||
                stack.ControlState == ResourceControlState.Processing))
        {
            return Result.Failure<IReadOnlyDictionary<Guid, StackReleaseStatus>>(
                new ConflictError("One or more stacks are currently processing another operation."));
        }

        var previousStatuses = stacks.ToDictionary(
            stack => stack.Id,
            stack => stack.CurrentStackRelease!.Status);

        try
        {
            foreach (var stack in stacks)
            {
                if (!stack.MarkProcessing(actorId) ||
                    !await unitOfWork.Stacks.UpdateProcessingAsync(
                        stack.Id,
                        stack.CurrentStackRelease!.Status,
                        stack.ControlState,
                        stack.ControlStartedAt,
                        stack.RowVersion,
                        checkRowVersion: true,
                        actorId,
                        cancellationToken))
                {
                    await unitOfWork.RollbackAsync();
                    RestoreClaimedEntities(stacks, previousStatuses);
                    return Result.Failure<IReadOnlyDictionary<Guid, StackReleaseStatus>>(
                        new ConflictError("One or more stacks changed while deletion was being claimed."));
                }
            }

            await unitOfWork.CommitAsync(cancellationToken);
            return Result.Success<IReadOnlyDictionary<Guid, StackReleaseStatus>>(previousStatuses);
        }
        catch
        {
            RestoreClaimedEntities(stacks, previousStatuses);
            throw;
        }
    }

    private async Task RollbackClaimsAsync(
        IReadOnlyCollection<Stack> stacks,
        IReadOnlyDictionary<Guid, StackReleaseStatus> previousStatuses,
        CancellationToken cancellationToken)
    {
        foreach (var stack in stacks)
        {
            var previousStatus = previousStatuses[stack.Id];
            stack.ReleaseProcessing(previousStatus);
            var restored = await unitOfWork.Stacks.UpdateProcessingAsync(
                stack.Id,
                previousStatus,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                cancellationToken);
            if (!restored)
            {
                await unitOfWork.RollbackAsync();
                return;
            }
        }

        await unitOfWork.CommitAsync(cancellationToken);
    }

    private static void RestoreClaimedEntities(
        IEnumerable<Stack> stacks,
        IReadOnlyDictionary<Guid, StackReleaseStatus> previousStatuses)
    {
        foreach (var stack in stacks)
            stack.ReleaseProcessing(previousStatuses[stack.Id]);
    }

    private void TryDeleteStackStorage(Stack stack)
    {
        try
        {
            DeleteStackStorage(stack);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Runtime containers and database state are already deleted. Leave stale files
            // behind rather than reporting a failed delete for a stack that no longer exists.
        }
    }

    private void DeleteStackStorage(Stack stack)
    {
        DeleteStackStoragePath(stack.Id.ToString("D"));
        DeleteStackStoragePath(SanitizeSegment(stack.Name));
        if (!string.IsNullOrWhiteSpace(stack.CurrentStackRelease?.Spec?.ProjectName))
        {
            DeleteStackStoragePath(SanitizeSegment(stack.CurrentStackRelease.Spec.ProjectName));
        }
    }

    private void DeleteStackStoragePath(string segment)
    {
        var stacksRoot = Path.GetFullPath(stackStoragePathProvider.StacksRoot);
        var rootWithSeparator = EnsureTrailingDirectorySeparator(stacksRoot);
        var targetPath = Path.GetFullPath(Path.Combine(rootWithSeparator, segment));

        if (!targetPath.StartsWith(rootWithSeparator, StringComparison.OrdinalIgnoreCase)
            || string.Equals(targetPath, stacksRoot, StringComparison.OrdinalIgnoreCase))
        {
            throw new IOException("Resolved stack storage path is outside the configured stacks directory.");
        }

        DeletePath(targetPath);
    }

    private static void DeletePath(string path)
    {
        if (!Directory.Exists(path) && !File.Exists(path))
        {
            return;
        }

        var attributes = File.GetAttributes(path);
        if ((attributes & FileAttributes.Directory) != 0)
        {
            var isSymlink = (attributes & FileAttributes.ReparsePoint) != 0;
            Directory.Delete(path, recursive: !isSymlink);
            return;
        }

        File.Delete(path);
    }

    private static string EnsureTrailingDirectorySeparator(string path)
        => path.EndsWith(Path.DirectorySeparatorChar.ToString(), StringComparison.Ordinal)
            ? path
            : path + Path.DirectorySeparatorChar;

    private static string SanitizeSegment(string value)
    {
        var invalidChars = Path.GetInvalidFileNameChars();
        var builder = new StringBuilder(value.Length);
        foreach (var c in value)
        {
            builder.Append(Array.IndexOf(invalidChars, c) >= 0 ? '_' : c);
        }

        return builder.Length == 0 ? "stack" : builder.ToString();
    }

    private async Task<Result<StackRuntimeDeletePlan?>> PrepareRuntimeContainerDeletionAsync(
        Stack stack,
        StackReleaseStatus previousStatus,
        CancellationToken cancellationToken)
    {
        if (stack.CurrentStackRelease is null || previousStatus == StackReleaseStatus.Created)
        {
            return Result.Success<StackRuntimeDeletePlan?>(null);
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out var platformError))
        {
            return Result.Failure<StackRuntimeDeletePlan?>(
                new NotFoundError(platformError?.Message ?? "Platform not found or disconnected."));
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var ownedContainerIds = await GetOwnedContainerIdsByStackLabelsAsync(
            connector,
            platform.Address,
            stack.Id,
            cancellationToken);
        if (ownedContainerIds.IsFailure(out var ownershipListError, out var containerIds))
        {
            return Result.Failure<StackRuntimeDeletePlan?>(ownershipListError);
        }

        if (containerIds.Count == 0)
        {
            string projectName;
            try
            {
                projectName = StackProjectNameResolver.Resolve(stack);
            }
            catch (InvalidOperationException ex)
            {
                return Result.Failure<StackRuntimeDeletePlan?>(new BadRequestError(ex.Message));
            }

            var projectContainerIds = await GetOwnedContainerIdsByProjectNameAsync(
                connector,
                platform.Address,
                projectName,
                stack.Id,
                cancellationToken);
            if (projectContainerIds.IsFailure(out var projectListError, out var projectOwnedContainerIds))
            {
                return Result.Failure<StackRuntimeDeletePlan?>(projectListError);
            }

            containerIds = projectOwnedContainerIds;
        }

        if (containerIds.Count == 0)
        {
            return Result.Success<StackRuntimeDeletePlan?>(null);
        }

        return Result.Success<StackRuntimeDeletePlan?>(new StackRuntimeDeletePlan(
            connector,
            new DeleteContainerCommand(
                containerIds,
                platform.Address,
                Volume: false,
                Force: true,
                Link: false)));
    }

    private static async Task<Result<IReadOnlyCollection<string>>> GetOwnedContainerIdsByStackLabelsAsync(
        IContainerConnector connector,
        string platformAddress,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        var listResult = await connector.ListContainersAsync(
            StackContainerOwnership.CreateStackOwnedContainerFilter(platformAddress, stackId),
            cancellationToken);

        if (listResult.IsFailure(out var listError, out var containers))
        {
            return Result.Failure<IReadOnlyCollection<string>>(new BadRequestError(listError.Message));
        }

        var containerIds = containers.Values
            .Select(container => container.Id)
            .Where(id => !string.IsNullOrWhiteSpace(id))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        return Result.Success<IReadOnlyCollection<string>>(containerIds);
    }

    private static async Task<Result<IReadOnlyCollection<string>>> GetOwnedContainerIdsByProjectNameAsync(
        IContainerConnector connector,
        string platformAddress,
        string projectName,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        var listResult = await connector.ListContainersAsync(
            StackContainerOwnership.CreateComposeProjectContainerFilter(platformAddress, projectName),
            cancellationToken);

        if (listResult.IsFailure(out var listError, out var containers))
        {
            return Result.Failure<IReadOnlyCollection<string>>(new BadRequestError(listError.Message));
        }

        var ownedContainerIds = new List<string>();
        foreach (var container in containers.Values)
        {
            var inspectResult = await connector.InspectAsync(
                new InspectContainerCommand(platformAddress, container.Id),
                cancellationToken);

            if (inspectResult.IsFailure(out var inspectError, out var inspect))
            {
                return Result.Failure<IReadOnlyCollection<string>>(new BadRequestError(
                    $"Unable to inspect stack container '{container.Name}' ({container.Id}): {inspectError.Message}"));
            }

            var labels = inspect.Config?.Labels ?? new Dictionary<string, string>();
            if (StackContainerOwnership.IsOwnedByStack(labels, stackId))
            {
                ownedContainerIds.Add(container.Id);
            }
        }

        return Result.Success<IReadOnlyCollection<string>>(
            ownedContainerIds.Distinct(StringComparer.OrdinalIgnoreCase).ToArray());
    }

    private sealed record StackRuntimeDeletePlan(
        IContainerConnector Connector,
        DeleteContainerCommand Command);
}
