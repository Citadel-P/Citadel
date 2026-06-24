using Application.Features.Deployments.Notifications;
using Application.Mappers;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;

namespace Application.Services;

internal interface IApplyStackService
{
    IAsyncEnumerable<StackStreamItem> ApplyAsync(
        Guid stackId,
        Guid actorId,
        IReadOnlyList<string>? serviceNames,
        bool pullImages,
        CancellationToken ct);
}

internal class ApplyStackService(
    IDbWorkQueue dbWorkQueue,
    IStackStreamManager stackHub,
    IServiceScopeFactory scopeFactory,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IStackConnector> stackConnectorFactory,
    IConnectorFactory<IContainerConnector> containerConnectorFactory) : IApplyStackService
{
    public async IAsyncEnumerable<StackStreamItem> ApplyAsync(
        Guid stackId,
        Guid actorId,
        IReadOnlyList<string>? serviceNames,
        bool pullImages,
        [EnumeratorCancellation] CancellationToken ct)
    {
        var stack = await LoadStack(stackId, ct);

        if (stack is null)
        {
            yield return StackStreamItem.FromStdErr($"❌ Stack with ID {stackId} not found.", 1);
            yield break;
        }

        if (stack.CurrentStackRelease?.Spec is null)
        {
            var message = $"❌ Stack with ID {stackId} has no spec defined.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out _))
        {
            var message = "❌ Platform not found or disconnected.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        var currentRelease = stack.CurrentStackRelease;

        if (currentRelease.Spec is not ManualStack manualStack)
        {
            var message = "Only manual stack specs are currently supported for stack apply.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        if (string.IsNullOrWhiteSpace(manualStack.ComposeFile))
        {
            var message = "❌ Manual stack compose file is required.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        string projectName;
        string? projectSetupError = null;
        try
        {
            projectName = StackProjectNameResolver.Resolve(stack);
        }
        catch (InvalidOperationException ex)
        {
            projectName = string.Empty;
            projectSetupError = $"❌ {ex.Message}";
        }

        if (projectSetupError is not null)
        {
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, projectSetupError, ct: ct);
            yield return StackStreamItem.FromStdErr(projectSetupError, 1);
            yield break;
        }

        string? registryAuth = null;
        string? registryName = null;
        string? registryHost = null;
        if (currentRelease.Spec.RegistryId is Guid registryId && registryId != Guid.Empty)
        {
            var registry = await LoadRegistry(registryId, ct);
            if (registry is null)
            {
                var message = $"❌ Registry with ID {registryId} not found.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            var host = registry.RegistryHost.Contains("://", StringComparison.Ordinal)
                ? registry.RegistryHost
                : $"https://{registry.RegistryHost}";

            registryHost = new Uri(host).Host.ToLowerInvariant();
            registryAuth = registry.Configuration.GetRegistryAuth(registryHost);
            registryName = registry.Name;
        }

        var knownStackContainerIds = await LoadStackContainerIds(stack.Id, ct);
        var collisionMessage = await ValidateProjectContainerOwnershipAsync(stack, platform, projectName, knownStackContainerIds, ct);
        if (!string.IsNullOrWhiteSpace(collisionMessage))
        {
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, collisionMessage, ct: ct);
            yield return StackStreamItem.FromStdErr($"❌ {collisionMessage}", 1);
            yield break;
        }

        var markResult = await MarkProcessingAsync(stack.Id, actorId, ct);
        if (!markResult.IsSuccess)
        {
            var message = markResult.ErrorMessage ?? "❌ Stack is already being processed.";
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, markResult.Stack!), ct);
        stack = markResult.Stack!;
        currentRelease = stack.CurrentStackRelease!;
        manualStack = (ManualStack)currentRelease.Spec;
        var composeFileContent = StackComposeLabelInjector.Inject(manualStack.ComposeFile, stack.Id, currentRelease.Id);

        var isServiceScopedApply = serviceNames is { Count: > 0 };
        yield return StackStreamItem.FromStdOut(isServiceScopedApply
            ? $"Applying stack services to {platform.Address}..."
            : $"Applying stack to {platform.Address}...");

        var connector = stackConnectorFactory.GetConnector(platform.ConnectorType);
        var command = BuildApplyCommand(
            stack,
            platform.Address,
            manualStack,
            composeFileContent,
            projectName,
            registryAuth,
            registryName,
            registryHost,
            serviceNames,
            pullImages);

        int? exitCode = null;
        var errorLogs = new List<string>(); 
        var enumerator = connector.StackApplyAsync(command, ct).GetAsyncEnumerator(ct);

        try
        {
            while (true)
            {
                var next = await TryReadNextAsync(enumerator);
                if (next.ErrorMessage is not null)
                {
                    await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, next.ErrorMessage, ct: ct);
                    yield return StackStreamItem.FromStdErr(next.ErrorMessage, exitCode ?? 1);
                    yield break;
                }

                if (!next.HasItem || next.Result is null)
                {
                    break;
                }

                var result = next.Result;

                if (!string.IsNullOrWhiteSpace(result.Message))
                {
                    if (result.Type == StackApplyEventType.SystemMessage)
                    {
                        yield return StackStreamItem.SystemMessage(result.Message, result.ExitCode ?? 0);
                    }
                    else
                    {
                        // Docker writes warnings and progress to stderr. 
                        // Only treat it as a critical failure message if it contains "error" or "failed"
                        if (result.Message.Contains("error", StringComparison.OrdinalIgnoreCase) ||
                            result.Message.Contains("failed", StringComparison.OrdinalIgnoreCase))
                        {
                            errorLogs.Add(result.Message);
                        }
                        else
                        {
                            yield return StackStreamItem.FromStdOut(result.Message);
                        }
                    }
                }

                if (result.ExitCode.HasValue)
                {
                    exitCode = result.ExitCode;
                    yield return StackStreamItem.Finished(result.ExitCode.Value);

                    if (result.ExitCode != 0)
                    {
                        var explicitFailure = errorLogs.Count > 0
                            ? string.Join(Environment.NewLine, errorLogs)
                            : $"❌ Pipeline command failed with exit code {result.ExitCode}.";

                        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, explicitFailure, ct: ct);
                        yield return StackStreamItem.FromStdErr($"❌ {explicitFailure}", result.ExitCode.Value);
                        yield break;
                    }
                }
            }
        }
        finally
        {
            await enumerator.DisposeAsync();
        }

        if (exitCode == 0)
        {
            var (errorMessage, containers) = await GetContainers(stack, platform, ct);
            if (!string.IsNullOrEmpty(errorMessage))
            {
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, errorMessage, ct: ct);
                yield return StackStreamItem.FromStdErr($"❌ {errorMessage}", exitCode ?? 1 );
                yield break;
            }
        
            await dbWorkQueue.EnqueueAsync(
                new StackSucceededWorkItem(
                    stack.Id,
                    actorId,
                    containers ?? [],
                    stackHub,
                    activityHub,
                    notificationQueue),
                ct);

            yield return StackStreamItem.SystemMessage("✅ Stack is now running.", 0);
            yield break;
        }

        var finalFailureMessage = errorLogs.Count > 0
            ? string.Join(Environment.NewLine, errorLogs)
            : (exitCode is int code ? $"❌ docker compose exited with code {code}." : "❌ Stack apply did not report a completion exit code.");

        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, finalFailureMessage, ct: ct);

        yield return StackStreamItem.FromStdErr(finalFailureMessage, exitCode ?? 1);
    }

    private async Task<(string? ErrorMessage, DockerContainer[]? Containers)> GetContainers(Stack stack, Domain.Contracts.Resources.PlatformCacheEntry platform, CancellationToken ct)
    {
        var filter = StackContainerOwnership.CreateOwnedContainerFilter(
            platform.Address,
            StackProjectNameResolver.Resolve(stack),
            stack.Id);

        var containerConnector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var containerListResult = await containerConnector.ListContainersAsync(filter, ct);

        return containerListResult.IsFailure(out var error, out var containerDic)
            ? (error.Message, null)
            : (null, containerDic.Values.Where(container => !string.IsNullOrWhiteSpace(container.Id)).ToArray());
    }

    private async Task<string?> ValidateProjectContainerOwnershipAsync(
        Stack stack,
        Domain.Contracts.Resources.PlatformCacheEntry platform,
        string projectName,
        HashSet<string> knownStackContainerIds,
        CancellationToken ct)
    {
        var filter = new ContainerFilterCommand(
            PlatformAddress: platform.Address,
            All: true,
            Filters: new Dictionary<string, IDictionary<string, bool>>
            {
                ["label"] = new Dictionary<string, bool>
                {
                    [$"{ComposeLabels.Project}={projectName}"] = true
                }
            });

        var containerConnector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var containerListResult = await containerConnector.ListContainersAsync(filter, ct);
        if (containerListResult.IsFailure(out var listError, out var containers))
        {
            return listError.Message;
        }

        foreach (var container in containers.Values)
        {
            var inspectResult = await containerConnector.InspectAsync(
                new InspectContainerCommand(platform.Address, container.Id),
                ct);

            if (inspectResult.IsFailure(out var inspectError, out var inspect))
            {
                return $"Unable to inspect existing compose project container '{container.Name}' ({container.Id}): {inspectError.Message}";
            }

            var labels = inspect.Config?.Labels ?? new Dictionary<string, string>();
            if (StackContainerOwnership.IsOwnedByStack(labels, stack.Id))
            {
                continue;
            }

            if (knownStackContainerIds.Contains(container.Id))
            {
                continue;
            }

            if (StackContainerOwnership.IsCitadelManaged(labels))
            {
                labels.TryGetValue(CitadelLabels.StackId, out var ownerStackId);
                return string.IsNullOrWhiteSpace(ownerStackId)
                    ? $"Docker Compose project '{projectName}' is already managed by another Citadel stack."
                    : $"Docker Compose project '{projectName}' is already managed by Citadel stack {ownerStackId}.";
            }

            return $"Docker Compose project '{projectName}' already has unmanaged containers on {platform.Address}. Choose another project name or remove container '{container.Name}'.";
        }

        return null;
    }

    private async Task<HashSet<string>> LoadStackContainerIds(Guid stackId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await uow.Stacks.GetContainerIdsAsync(stackId, ct);
        return containers.ToHashSet(StringComparer.Ordinal);
    }

    private StackApplyCommand BuildApplyCommand(Stack stack, string platformAddress, ManualStack manualStack,
        string composeFileContent, string projectName,
        string? registryAuth, string? registryName, string? registryHost,
        IReadOnlyList<string>? serviceNames,
        bool pullImages)
        => new(
            PlatformAddress: platformAddress,
            StackName: stack.Name,
            ComposeFileContent: composeFileContent,
            ProjectName: projectName,
            EnvironmentFilePath: manualStack.EnvFilePath,
            EnvironmentVariables: manualStack.EnvVars,
            PreDeploy: manualStack.PreDeploy,
            PostDeploy: manualStack.PostDeploy,
            RegistryAuth: registryAuth,
            RegistryName: registryName,
            RegistryHost: registryHost,
            DestroyBeforeDeploy: manualStack.DestroyBeforeDeploy && serviceNames is not { Count: > 0 },
            Spec: manualStack,
            ServiceNames: serviceNames,
            PullImages: pullImages);


    private static async Task<(bool HasItem, StackApplyResult? Result, string? ErrorMessage)> TryReadNextAsync(IAsyncEnumerator<StackApplyResult> enumerator)
    {
        try
        {
            var hasItem = await enumerator.MoveNextAsync();
            return hasItem
                ? (true, enumerator.Current, null)
                : (false, null, null);
        }
        catch (Exception ex)
        {
            return (false, null, $"Stack apply failed: {ex.Message}");
        }
    }

    private async Task<(bool IsSuccess, Stack? Stack, string? ErrorMessage)> MarkProcessingAsync(Guid stackId, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null)
        {
            return (false, null, $"Stack with ID {stackId} not found.");
        }

        if (!stack.PrepareReleaseForApply(actorId))
        {
            return (false, null, "Stack has no release to apply.");
        }

        if (!stack.MarkProcessing(actorId))
        {
            return (false, null, "Stack is already being processed.");
        }

        stack.PartialUpdate(StackReleaseStatus.Applying);
        await uow.Stacks.UpdateAsync(stack, ct);
        await uow.CommitAsync(ct);
        return (true, stack, null);
    }

    private async Task<Stack?> LoadStack(Guid id, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Stacks.GetAsync(id, ct);
    }

    private async Task<Registry?> LoadRegistry(Guid registryId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Registries.GetAsync(registryId, ct);
    }

    private ValueTask EnqueueStatus(Guid stackId, Guid actorId, StackReleaseStatus status, string? message, CancellationToken ct = default)
        => dbWorkQueue.EnqueueAsync(
            new UpdateStackStatusWorkItem(
                stackId,
                actorId,
                status,
                message,
                stackHub,
                activityHub,
                notificationQueue),
            ct);
}

internal sealed class UpdateStackStatusWorkItem(Guid stackId, Guid actorId, StackReleaseStatus status, string? message, IStackStreamManager stackHub,
    IActivityStreamManager activityHub, INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null) return;

        stack.ReleaseProcessing(status);
        await uow.Stacks.UpdateAsync(stack, ct);

        // Add activity event
        ActivityEvent? activity = null;
        if (status == StackReleaseStatus.Failed)
        {
            activity = new ActivityEvent(
                            actorId: actorId,
                            resourceId: stack.Id,
                            platformId: stack.CurrentStackRelease?.PlatformId ?? Guid.Empty,
                            resourceName: stack.Name,
                            status: ActivityStatus.Failure,
                            eventType: ActivityEventType.StackApplied,
                            info: new StackApplied(stack.ToSnapshot(), new StackResultSnapshot(null, Helpers.RemoveAnsiSequences(message ?? "")))
                            );

            await uow.ActivityEventRepository.AddAsync(activity, ct);
            stack.AssignActivityEvent(activity);
        }

        await uow.CommitAsync(ct);

        // Push notifications

        var stackWorkItem = new StackNotificationWorkItem(stackHub, stack);
        await notificationQueue.EnqueueAsync(stackWorkItem, ct);

        if (activity is not null)
        {
            var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
            await notificationQueue.EnqueueAsync(activityWorkItem, ct);
        }
    }
}

internal sealed class StackSucceededWorkItem(
    Guid stackId,
    Guid actorId,
    DockerContainer[] dockerContainers,
    IStackStreamManager stackHub,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack?.CurrentStackRelease is null) return;

        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        var platformId = stack.CurrentStackRelease.PlatformId;
        var images = await uow.Images.GetByPlatformIdAsync(platformId, ct);
        var existingContainers = (await uow.Containers.GetByPlatformIdAsync(platformId, ct))
            .ToDictionary(
                container => container.DockerContainerId,
                container => container,
                StringComparer.OrdinalIgnoreCase);

        var upserts = new List<Container>();
        foreach (var dockerContainer in dockerContainers)
        {
            var imageId = images.FirstOrDefault(image =>
                image.DockerImageId == dockerContainer.ImageId &&
                image.PlatformId == platformId)?.Id;

            if (existingContainers.TryGetValue(dockerContainer.Id, out var existingContainer))
            {
                existingContainer.PartialUpdate(
                    name: dockerContainer.Name,
                    imageId: imageId,
                    dockerImageId: dockerContainer.ImageId,
                    state: dockerContainer.State,
                    dockerStack: dockerContainer.Stack,
                    created: dockerContainer.Created,
                    ports: dockerContainer.Ports,
                    stackId: stack.Id);

                upserts.Add(existingContainer);
                continue;
            }

            var container = dockerContainer.Map(platformId, imageId);
            container.PartialUpdate(stackId: stack.Id);
            upserts.Add(container);
        }

        await uow.Containers.BulkUpsertAsync(upserts, ct);
        await uow.Stacks.UpdateAsync(stack, ct);

        var containerIds = upserts.Select(container => container.DockerContainerId).ToArray();
        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: stack.Id,
                        platformId: stack.CurrentStackRelease?.PlatformId,
                        resourceName: stack.Name,
                        status: ActivityStatus.Success,
                        eventType: ActivityEventType.StackApplied,
                        info: new StackApplied(stack.ToSnapshot(), new StackResultSnapshot(containerIds, "Stack applied successfully."))
                        );

        await uow.ActivityEventRepository.AddAsync(activity, ct);

        await uow.CommitAsync(ct);

        stack.AssignActivityEvent(activity);
        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, stack), ct);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct)), ct);
    }
}

internal sealed class StackNotificationWorkItem(IStackStreamManager stackHub, Stack stack, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => stackHub.SendStackInfo(stack, action);
}
