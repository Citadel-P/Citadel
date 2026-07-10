using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
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
    IStackStoragePathProvider stackStoragePathProvider) : ICommandHandler<DeleteStacks, Result>
{
    public async ValueTask<Result> Handle(DeleteStacks command, CancellationToken cancellationToken)
    {
        var stacks = (await unitOfWork.Stacks.GetAllAsync(command.Ids, cancellationToken))?.ToArray();
        if (stacks == null || stacks.Length == 0)
        {
            return Result.Failure(new NotFoundError("No stacks found matching the provided IDs."));
        }

        foreach (var stack in stacks)
        {
            var cleanupResult = await DeleteRuntimeContainersAsync(stack, cancellationToken);
            if (cleanupResult.IsFailure())
            {
                return cleanupResult;
            }

        }

        var platformIds = stacks
            .Select(stack => stack.CurrentStackRelease?.PlatformId)
            .Where(platformId => platformId.HasValue)
            .Select(platformId => platformId!.Value)
            .Distinct()
            .ToArray();

        await unitOfWork.Stacks.RemoveRangeAsync(stacks.Select(stack => stack.Id), cancellationToken);
        var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatByIdsAsync(platformIds, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        foreach (var stack in stacks)
        {
            TryDeleteStackStorage(stack);
        }

        foreach (var stack in stacks)
        {
            await stackHub.SendStackInfo(stack, "delete");
        }

        foreach (var platform in platforms)
        {
            await platformHub.PushPlatformUpdate(platform);
        }

        return Result.Success();
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

    private async Task<Result> DeleteRuntimeContainersAsync(Stack stack, CancellationToken cancellationToken)
    {
        if (stack.CurrentStackRelease is null || stack.CurrentStackRelease.Status == StackReleaseStatus.Created)
        {
            return Result.Success();
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out var platformError))
        {
            return Result.Failure(new NotFoundError(platformError?.Message ?? "Platform not found or disconnected."));
        }

        string projectName;
        try
        {
            projectName = StackProjectNameResolver.Resolve(stack);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure(new BadRequestError(ex.Message));
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var listResult = await connector.ListContainersAsync(
            new ContainerFilterCommand(
                PlatformAddress: platform.Address,
                All: true,
                Filters: new Dictionary<string, IDictionary<string, bool>>
                {
                    ["label"] = new Dictionary<string, bool>
                    {
                        [$"{ComposeLabels.Project}={projectName}"] = true
                    }
                }),
            cancellationToken);

        if (listResult.IsFailure(out var listError, out var containers))
        {
            return Result.Failure(new BadRequestError(listError.Message));
        }

        var ownedContainerIds = new List<string>();
        foreach (var container in containers.Values)
        {
            var inspectResult = await connector.InspectAsync(
                new InspectContainerCommand(platform.Address, container.Id),
                cancellationToken);

            if (inspectResult.IsFailure(out var inspectError, out var inspect))
            {
                return Result.Failure(new BadRequestError(
                    $"Unable to inspect stack container '{container.Name}' ({container.Id}): {inspectError.Message}"));
            }

            var labels = inspect.Config?.Labels ?? new Dictionary<string, string>();
            if (StackContainerOwnership.IsOwnedByStack(labels, stack.Id))
            {
                ownedContainerIds.Add(container.Id);
            }
        }

        if (ownedContainerIds.Count == 0)
        {
            return Result.Success();
        }

        return await connector.DeleteAsync(
            new DeleteContainerCommand(
                ownedContainerIds.Distinct(StringComparer.OrdinalIgnoreCase),
                platform.Address,
                Volume: false,
                Force: true,
                Link: false),
            cancellationToken);
    }
}
