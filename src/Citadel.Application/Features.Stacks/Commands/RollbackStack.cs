using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Apply, ResourceIdProperty = nameof(RollbackStack.StackId))]
public sealed record RollbackStack(Guid StackId, Guid ReleaseId) : IStreamCommand<StackStreamItem>;

internal sealed class RollbackStackHandler(
    IUnitOfWork unitOfWork,
    IApplyStackService applyStackService,
    IUserContextAccessor userContext) : IStreamCommandHandler<RollbackStack, StackStreamItem>
{
    public async IAsyncEnumerable<StackStreamItem> Handle(
        RollbackStack request,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;

        var prepare = await PrepareRollbackReleaseAsync(request.StackId, request.ReleaseId, actorId, cancellationToken);
        if (!prepare.Success)
        {
            yield return StackStreamItem.FromStdErr(prepare.Error ?? "Rollback failed.", 1);
            yield break;
        }

        yield return StackStreamItem.SystemMessage($"Rollback release prepared from version {prepare.Version}.", 0);

        await foreach (var streamItem in applyStackService.ApplyAsync(
            request.StackId,
            actorId,
            serviceNames: null,
            pullImages: false,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Rollback,
            prepare.PreviousStackSnapshot,
            cancellationToken))
        {
            yield return streamItem;
        }
    }

    private async Task<(bool Success, string? Error, string? Version, StackSnapshot? PreviousStackSnapshot)> PrepareRollbackReleaseAsync(
        Guid stackId,
        Guid releaseId,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(stackId, cancellationToken);
        if (stack?.CurrentStackRelease is null)
        {
            return (false, $"Stack with ID {stackId} does not exist.", null, null);
        }

        if (stack.ControlState == ResourceControlState.Processing)
        {
            return (false, "Stack is already being processed.", null, null);
        }

        if (stack.CurrentStackReleaseId == releaseId)
        {
            return (false, "Selected release is already the current release.", null, null);
        }

        var releases = await unitOfWork.Stacks.GetReleasesByStackIdAsync(stackId, cancellationToken);
        var release = releases.FirstOrDefault(item => item.Id == releaseId);
        if (release is null)
        {
            return (false, $"Release with ID {releaseId} does not exist for stack {stackId}.", null, null);
        }

        if (release.Version == stack.CurrentStackRelease.Version)
        {
            return (false, "Only releases older than the current version can be rolled back.", null, null);
        }

        if (!CanRollback(release))
        {
            return (false, "Only previous healthy releases can be rolled back.", null, null);
        }

        var usesMountedSecrets = release.ResourceBindings?.Any(static binding =>
                binding.Kind == ResourceBindingKind.Secret
                && binding.SecretDeliveryMode == SecretDeliveryMode.MountedFile) == true;
        IReadOnlyList<StackReleaseSwarmResource> retainedSwarmResources = [];
        var platform = stack.CurrentStackRelease.Platform;
        if (platform is null && usesMountedSecrets)
        {
            platform = await unitOfWork.Platforms.GetByIdAsync(
                stack.CurrentStackRelease.PlatformId,
                cancellationToken);
        }
        if (platform?.PlatformDescriptor.Type == PlatformType.DockerSwarm)
        {
            retainedSwarmResources = await unitOfWork.Stacks.GetReleaseSwarmResourcesAsync(
                release.Id,
                cancellationToken) ?? [];
            if (usesMountedSecrets
                && !retainedSwarmResources.Any(static resource =>
                    resource.Kind == StackReleaseSwarmResourceKind.Secret
                    && resource.Mounts.Count > 0))
            {
                return (false, "Exact rollback is unavailable because this release's versioned Docker Secrets are outside the retention window or were not recorded. Create a new reviewed release instead.", null, null);
            }
        }

        var rollbackSpec = CreateRollbackSpec(release);
        if (rollbackSpec is null)
        {
            return (false, "Selected release does not contain enough Git source metadata to roll back.", null, null);
        }

        var previousStackSnapshot = stack.ToSnapshot();
        if (!stack.PrepareRollbackRelease(release, rollbackSpec, actorId))
        {
            return (false, "Rollback release could not be prepared.", null, null);
        }

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        if (retainedSwarmResources.Count > 0)
        {
            await unitOfWork.Stacks.ReplaceReleaseSwarmResourcesAsync(
                stack.CurrentStackReleaseId,
                [.. retainedSwarmResources.Select(resource => resource.ForRelease(stack.CurrentStackReleaseId))],
                cancellationToken);
        }
        await unitOfWork.CommitAsync(cancellationToken);

        return (true, null, release.Version, previousStackSnapshot);
    }

    private static bool CanRollback(StackRelease release)
        => release.IsRollbackCandidate();

    private static StackSpec? CreateRollbackSpec(StackRelease release)
        => release.Spec switch
        {
            GitStack gitStack => CreateGitRollbackSpec(gitStack, release.Source),
            ManualStack manualStack => manualStack,
            _ => null
        };

    private static GitStack? CreateGitRollbackSpec(GitStack gitStack, StackReleaseSource? source)
    {
        if (source is null)
        {
            return CreatePinnedGitRollbackSpec(gitStack);
        }

        if (source.SourceType != StackSource.Git
            || string.IsNullOrWhiteSpace(source.ResolvedCommitSha))
        {
            return null;
        }

        var repositoryId = source.GitRepositoryId ?? gitStack.GitRepoId;
        if (repositoryId == Guid.Empty)
        {
            return null;
        }

        var branch = string.IsNullOrWhiteSpace(source.Branch)
            ? gitStack.Branch
            : source.Branch;
        if (string.IsNullOrWhiteSpace(branch))
        {
            return null;
        }

        var composePaths = source.ComposePaths is { Count: > 0 }
            ? source.ComposePaths.ToList()
            : gitStack.ComposePaths;
        if (composePaths is not { Count: > 0 })
        {
            return null;
        }

        var composeEnvFiles = (source.ComposeEnvFilesFromRepo ?? source.EnvFilePaths)?.ToList();

        return gitStack with
        {
            GitRepoId = repositoryId,
            Branch = branch,
            CommitSha = source.ResolvedCommitSha,
            ComposePaths = composePaths,
            WorkingDirectory = source.WorkingDirectory ?? gitStack.WorkingDirectory,
            ComposeEnvFilesFromRepo = composeEnvFiles,
            AdditionalEnvFileFromRepo = null,
            WatchPaths = source.WatchPaths?.ToList() ?? gitStack.WatchPaths
        };
    }

    private static GitStack? CreatePinnedGitRollbackSpec(GitStack gitStack)
    {
        if (gitStack.GitRepoId == Guid.Empty
            || string.IsNullOrWhiteSpace(gitStack.Branch)
            || string.IsNullOrWhiteSpace(gitStack.CommitSha)
            || gitStack.ComposePaths is not { Count: > 0 })
        {
            return null;
        }

        return gitStack;
    }
}
