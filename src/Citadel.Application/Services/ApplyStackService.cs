using Application.Features.Deployments.Notifications;
using Application.Services.Builds;
using Application.Mappers;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;

namespace Application.Services;

public interface IApplyStackService
{
    IAsyncEnumerable<StackStreamItem> ApplyAsync(
        Guid stackId,
        Guid actorId,
        IReadOnlyList<string>? serviceNames,
        bool pullImages,
        bool recreate,
        bool waitForCompletion,
        StackApplyOperation operation,
        StackSnapshot? previousStackSnapshot,
        CancellationToken ct);
}

public enum StackApplyOperation
{
    Apply,
    Rollback
}

internal class ApplyStackService(
    IDbWorkQueue dbWorkQueue,
    IStackStreamManager stackHub,
    IServiceScopeFactory scopeFactory,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IStackConnector> stackConnectorFactory,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IGitStackMaterializer gitStackMaterializer,
    IResourceBindingResolver resourceBinderResolver,
    ISecretRedactor secretRedactor,
    IAlertService alertService,
    IStackBuildImageBindingResolver stackBuildImageBindingResolver,
    IStackOperationBarrier? operationBarrier = null) : IApplyStackService
{
    private readonly IStackOperationBarrier _operationBarrier = operationBarrier ?? new NoOpStackOperationBarrier();

    public async IAsyncEnumerable<StackStreamItem> ApplyAsync(
        Guid stackId,
        Guid actorId,
        IReadOnlyList<string>? serviceNames,
        bool pullImages,
        bool recreate,
        bool waitForCompletion,
        StackApplyOperation operation,
        StackSnapshot? previousStackSnapshot,
        [EnumeratorCancellation] CancellationToken ct)
    {
        var stack = await LoadStack(stackId, ct);

        if (stack is null)
        {
            yield return StackStreamItem.FromStdErr($"Stack with ID {stackId} not found.", 1);
            yield break;
        }

        if (stack.ControlState == ResourceControlState.Processing)
        {
            yield return StackStreamItem.FromStdErr("Stack is already being processed.", 1);
            yield break;
        }

        if (stack.CurrentStackRelease?.Spec is null)
        {
            var message = $"Stack with ID {stackId} has no spec defined.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out _))
        {
            var message = "Platform not found or disconnected.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        var currentRelease = stack.CurrentStackRelease;

        if (currentRelease.Spec is not ManualStack and not GitStack)
        {
            var message = "Unsupported stack source.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        if (currentRelease.Spec is ManualStack manualStack && string.IsNullOrWhiteSpace(manualStack.ComposeFile))
        {
            var message = "Manual stack compose file is required.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
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
            projectSetupError = ex.Message;
        }

        if (projectSetupError is not null)
        {
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, projectSetupError, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
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
                var message = $"Registry with ID {registryId} not found.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
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
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, collisionMessage, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(collisionMessage, 1);
            yield break;
        }

        var markResult = await MarkProcessingAsync(stack.Id, actorId, ct);
        if (!markResult.IsSuccess)
        {
            var message = markResult.ErrorMessage ?? "Stack is already being processed.";
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, markResult.Stack!), ct);
        stack = markResult.Stack!;
        var operationRowVersion = markResult.OperationRowVersion!.Value;
        await _operationBarrier.WaitAsync(
            StackOperationCheckpoint.ProcessingClaimed,
            stack.Id,
            operationRowVersion,
            ct);
        currentRelease = stack.CurrentStackRelease!;
        var stackSpec = currentRelease.Spec;
        var buildImageBindings = await stackBuildImageBindingResolver.ResolveAsync(stackSpec.BuildImageBindings, ct);
        if (buildImageBindings.IsFailure(out var buildBindingError, out var resolvedBuildBindings))
        {
            var message = buildBindingError.Message;
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        foreach (var message in resolvedBuildBindings.Messages)
            yield return StackStreamItem.SystemMessage(message, 0);

        string? composeFileContent;
        string? environmentFilePath;
        IReadOnlyList<string>? environmentVariables;
        IReadOnlyList<string>? sourceEnvironmentVariables = null;
        string? sourceWorkingDirectory = null;
        IReadOnlyList<string>? sourceComposeFilePaths = null;
        IReadOnlyList<string>? sourceEnvFilePaths = null;
        string? labelsOverrideFilePath = null;
        string? generatedFilesDirectory = null;
        string? gitSnapshotRoot = null;
        IReadOnlyList<string>? secretTargetServiceNames = null;
        StackReleaseSource? releaseSource = null;
        yield return StackStreamItem.SystemMessage("Resolving stack variables and secrets...", 0);
        var configurationResult = await resourceBinderResolver.ResolveAsync(ResourceBindingScope.Stack, stack.Id, ct);
        if (configurationResult.IsFailure(out var configurationError, out var resolvedConfiguration))
        {
            var message = configurationError.Message;
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
            await ProcessConfigurationFailureAlertAsync(stack.Id, stack.Name, message, ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        if (stackSpec is ManualStack currentManualStack)
        {
            var composeWithBuildImages = stackBuildImageBindingResolver.ApplyToComposeContent(
                currentManualStack.ComposeFile,
                resolvedBuildBindings.Bindings);
            composeFileContent = StackComposeLabelInjector.Inject(composeWithBuildImages, stack.Id, currentRelease.Id);
            environmentFilePath = currentManualStack.EnvFilePath;
            environmentVariables = [];
            secretTargetServiceNames = StackComposeParser
                .ParseServices(stack.Id, currentRelease.Id, currentManualStack.ComposeFile)
                .Keys
                .ToArray();
        }
        else if (stackSpec is GitStack gitStack)
        {
            var gitRepository = await LoadGitRepository(gitStack.GitRepoId, ct);
            if (gitRepository is null)
            {
                var message = $"Git repository with ID {gitStack.GitRepoId} not found.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            var materialization = await gitStackMaterializer.MaterializeAsync(stack, gitStack, gitRepository, ct);
            if (materialization.IsFailure(out var materializationError, out var payload))
            {
                var message = materializationError.Message;
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            composeFileContent = null;
            environmentFilePath = payload.EnvFilePath;
            sourceEnvironmentVariables = payload.EnvironmentVariables;
            environmentVariables = sourceEnvironmentVariables;
            sourceWorkingDirectory = payload.SourceWorkingDirectory;
            sourceComposeFilePaths = payload.SourceComposeFilePaths;
            sourceEnvFilePaths = payload.SourceEnvFilePaths;
            labelsOverrideFilePath = payload.LabelsOverrideFilePath;
            generatedFilesDirectory = payload.GeneratedFilesDirectory;
            if (resolvedBuildBindings.Bindings.Count > 0)
            {
                generatedFilesDirectory ??= Path.Combine(sourceWorkingDirectory ?? Path.GetTempPath(), ".citadel");
                Directory.CreateDirectory(generatedFilesDirectory);
                var buildOverrideFilePath = Path.Combine(generatedFilesDirectory, "build-images.override.yml");
                await File.WriteAllTextAsync(
                    buildOverrideFilePath,
                    stackBuildImageBindingResolver.CreateComposeOverride(resolvedBuildBindings.Bindings),
                    ct);
                sourceComposeFilePaths = [.. sourceComposeFilePaths, buildOverrideFilePath];
            }

            gitSnapshotRoot = payload.SnapshotRoot;
            secretTargetServiceNames = StackComposeParser
                .ParseServices(stack.Id, currentRelease.Id, [.. payload.SourceComposeFilePaths.Select(File.ReadAllText)])
                .Keys
                .ToArray();
            releaseSource = new StackReleaseSource(
                SourceType: StackSource.Git,
                GitRepositoryId: gitRepository.Id,
                GitRepositoryName: gitRepository.Name,
                Branch: payload.SourceBranch,
                RequestedCommitSha: gitStack.CommitSha,
                ResolvedCommitSha: payload.ResolvedCommitSha,
                ComposePaths: payload.ComposePaths,
                EnvFilePaths: payload.EnvFilePaths,
                GitRepositoryUrl: GitRepositoryUrlSanitizer.Sanitize(gitRepository.Url),
                WorkingDirectory: Path.GetRelativePath(payload.SnapshotRoot, payload.SourceWorkingDirectory).Replace('\\', '/'),
                WatchPaths: payload.WatchPaths,
                ComposeEnvFilesFromRepo: payload.EnvFilePaths);
        }
        else
        {
            var message = "Unsupported stack source.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        var referencedConfigurationKeys = await GetReferencedConfigurationKeysAsync(
            composeFileContent,
            sourceComposeFilePaths,
            ct);
        var selectedConfiguration = SelectStackApplyConfiguration(
            resolvedConfiguration,
            referencedConfigurationKeys);
        var secretFiles = BuildMountedSecretFiles(selectedConfiguration);
        if (secretFiles.Count > 0 && secretTargetServiceNames is not { Count: > 0 })
        {
            var message = "Mounted file secrets require at least one Compose service.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: operationRowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

        environmentVariables = sourceEnvironmentVariables is { Count: > 0 }
            ? [.. sourceEnvironmentVariables, .. selectedConfiguration.EnvironmentVariables]
            : selectedConfiguration.EnvironmentVariables;

        yield return StackStreamItem.SystemMessage(
            ResourceBindingApplyMessageBuilder.BuildComposeInterpolationMessage(
                selectedConfiguration,
                referencedConfigurationKeys,
                sourceEnvFilePaths?.Count ?? 0),
            0);

        var isServiceScopedApply = serviceNames is { Count: > 0 };
        yield return StackStreamItem.FromStdOut(isServiceScopedApply
            ? $"Applying stack services to {platform.Address}..."
            : $"Applying stack to {platform.Address}...");

        var connector = stackConnectorFactory.GetConnector(platform.ConnectorType);
        var command = BuildApplyCommand(
            stack,
            platform.Address,
            stackSpec,
            composeFileContent,
            projectName,
            environmentFilePath,
            environmentVariables,
            registryAuth,
            registryName,
            registryHost,
            serviceNames,
            pullImages,
            recreate,
            sourceWorkingDirectory,
            sourceComposeFilePaths,
            sourceEnvFilePaths,
            labelsOverrideFilePath,
            generatedFilesDirectory,
            secretFiles,
            secretTargetServiceNames);

        int? exitCode = null;
        StackReleaseStatus? composeStatus = null;
        var errorLogs = new List<string>(); 
        var enumerator = connector.StackApplyAsync(command, ct).GetAsyncEnumerator(ct);

        try
        {
            while (true)
            {
                var next = await TryReadNextAsync(enumerator);
                if (next.ErrorMessage is not null)
                {
                    var safeError = secretRedactor.Redact(next.ErrorMessage, selectedConfiguration.RedactionValues);
                    await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                    await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, safeError, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                    yield return StackStreamItem.FromStdErr(safeError, exitCode ?? 1);
                    yield break;
                }

                if (!next.HasItem || next.Result is null)
                {
                    break;
                }

                var result = next.Result;
                if (result.StackStatus is StackReleaseStatus reportedStatus)
                {
                    composeStatus = reportedStatus;
                }

                if (!string.IsNullOrWhiteSpace(result.Message))
                {
                    var safeMessage = secretRedactor.Redact(result.Message, selectedConfiguration.RedactionValues);
                    if (result.Type == StackApplyEventType.SystemMessage)
                    {
                        yield return StackStreamItem.SystemMessage(safeMessage, result.ExitCode ?? 0);
                    }
                    else
                    {
                        // Docker writes warnings and progress to stderr. 
                        // Only treat it as a critical failure message if it contains "error" or "failed"
                        if (safeMessage.Contains("error", StringComparison.OrdinalIgnoreCase) ||
                            safeMessage.Contains("failed", StringComparison.OrdinalIgnoreCase))
                        {
                            errorLogs.Add(safeMessage);
                        }
                        else
                        {
                            yield return StackStreamItem.FromStdOut(safeMessage);
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
                            : $"Pipeline command failed with exit code {result.ExitCode}.";

                        await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, explicitFailure, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                        yield return StackStreamItem.FromStdErr(explicitFailure, result.ExitCode.Value);
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
                await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, errorMessage, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(errorMessage, exitCode ?? 1 );
                yield break;
            }

            var volumeBindings = await ResolveVolumeBindingsAsync(stack, currentRelease, platform, containers ?? [], ct);
            if (!string.IsNullOrWhiteSpace(volumeBindings.Warning))
            {
                yield return StackStreamItem.SystemMessage(volumeBindings.Warning, 0);
            }

            if (!string.IsNullOrWhiteSpace(gitSnapshotRoot))
            {
                string? sourceSnapshotWarning = null;
                try
                {
                    await gitStackMaterializer.ActivateCurrentAsync(stack.Id, gitSnapshotRoot, ct);
                    var retainedReleaseIds = await LoadRetainedGitSnapshotReleaseIdsAsync(stack.Id, stack.CurrentStackReleaseId, ct);
                    await gitStackMaterializer.PruneSnapshotsAsync(stack.Id, retainedReleaseIds, ct);
                }
                catch (Exception ex)
                {
                    sourceSnapshotWarning = $"Git source snapshot maintenance failed: {ex.Message}";
                }

                if (sourceSnapshotWarning is not null)
                {
                    yield return StackStreamItem.SystemMessage(sourceSnapshotWarning, 0);
                }
            }
        
            await _operationBarrier.WaitAsync(
                StackOperationCheckpoint.ExternalStateCaptured,
                stack.Id,
                operationRowVersion,
                ct);

            var workItem = new StackSucceededWorkItem(
                stack.Id,
                actorId,
                containers ?? [],
                composeStatus,
                stackHub,
                activityHub,
                notificationQueue,
                operation,
                previousStackSnapshot,
                releaseSource,
                selectedConfiguration.SnapshotEntries,
                volumeBindings.Bindings,
                ResolveAppliedBuildImages(
                    resolvedBuildBindings.Bindings,
                    serviceNames,
                    secretTargetServiceNames),
                operationRowVersion);

            if (waitForCompletion)
            {
                await dbWorkQueue.EnqueueAndWaitAsync(workItem, ct);
                if (!workItem.Applied)
                {
                    yield return StackStreamItem.FromStdErr(
                        "Stack operation was superseded before its result could be committed.",
                        1);
                    yield break;
                }
            }
            else
            {
                await dbWorkQueue.EnqueueAsync(workItem, ct);
            }

            yield return StackStreamItem.SystemMessage(
                GetStackAppliedMessage(composeStatus),
                0,
                composeStatus);
            yield break;
        }

        var finalFailureMessage = errorLogs.Count > 0
            ? string.Join(Environment.NewLine, errorLogs)
            : (exitCode is int code ? $"docker compose exited with code {code}." : "Stack apply did not report a completion exit code.");

        await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, finalFailureMessage, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);

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
        if (containerListResult.IsSuccess(out var ownedContainers, out _))
        {
            var containers = ownedContainers.Values
                .Where(container => !string.IsNullOrWhiteSpace(container.Id))
                .ToArray();

            if (containers.Length > 0)
            {
                return (null, containers);
            }
        }

        var projectFilter = StackContainerOwnership.CreateComposeProjectContainerFilter(
            platform.Address,
            StackProjectNameResolver.Resolve(stack));
        containerListResult = await containerConnector.ListContainersAsync(projectFilter, ct);

        return containerListResult.IsFailure(out var error, out var containerDic)
            ? (error.Message, null)
            : (null, containerDic.Values.Where(container => !string.IsNullOrWhiteSpace(container.Id)).ToArray());
    }

    private async Task<(string? Warning, IReadOnlyList<StackReleaseVolumeBinding> Bindings)> ResolveVolumeBindingsAsync(
        Stack stack,
        StackRelease release,
        Domain.Contracts.Resources.PlatformCacheEntry platform,
        IReadOnlyCollection<DockerContainer> containers,
        CancellationToken ct)
    {
        if (containers.Count == 0)
            return (null, []);

        var composeVolumes = release.Spec is ManualStack manual
            ? StackComposeParser.ParseVolumes(manual.ComposeFile)
            : new StackComposeVolumeResolution([], [], false);
        var declared = composeVolumes.DeclaredVolumes.Select(static volume => volume.Name).ToHashSet(StringComparer.Ordinal);
        var external = composeVolumes.DeclaredVolumes
            .Where(static volume => volume.IsExternal)
            .Select(static volume => volume.Name)
            .ToHashSet(StringComparer.Ordinal);
        var references = composeVolumes.ServiceVolumeReferences.ToHashSet(StringComparer.Ordinal);
        var projectName = StackProjectNameResolver.Resolve(stack);
        var connector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var bindings = new Dictionary<string, StackReleaseVolumeBinding>(StringComparer.Ordinal);
        var inspectFailures = 0;

        foreach (var container in containers)
        {
            var inspect = await connector.InspectAsync(
                new InspectContainerCommand(platform.Address, container.Id),
                ct);

            if (!inspect.IsSuccess(out var info, out _))
            {
                inspectFailures++;
                continue;
            }

            foreach (var mount in info.Mounts)
            {
                if (!string.Equals(mount.Type, "volume", StringComparison.OrdinalIgnoreCase)
                    || string.IsNullOrWhiteSpace(mount.Name))
                {
                    continue;
                }

                var composeName = ResolveComposeVolumeName(mount.Name, projectName, declared, references);
                var isExternal = external.Contains(mount.Name)
                                 || (composeName is not null && external.Contains(composeName));
                var isAnonymous = composeName is null && !isExternal;
                var binding = new StackReleaseVolumeBinding(
                    release.Id,
                    release.PlatformId,
                    mount.Name,
                    composeName,
                    isExternal,
                    isAnonymous);

                bindings.TryAdd(binding.VolumeName, binding);
            }
        }

        var warning = inspectFailures == 0
            ? null
            : $"Could not inspect {inspectFailures} stack container{(inspectFailures == 1 ? string.Empty : "s")} while recording backup volume bindings.";

        return (warning, [.. bindings.Values.OrderBy(static binding => binding.VolumeName, StringComparer.Ordinal)]);
    }

    private static string? ResolveComposeVolumeName(
        string dockerVolumeName,
        string projectName,
        IReadOnlySet<string> declared,
        IReadOnlySet<string> references)
    {
        if (declared.Contains(dockerVolumeName) || references.Contains(dockerVolumeName))
            return dockerVolumeName;

        var prefix = projectName + "_";
        if (!dockerVolumeName.StartsWith(prefix, StringComparison.Ordinal))
            return null;

        var candidate = dockerVolumeName[prefix.Length..];
        return declared.Contains(candidate) || references.Contains(candidate)
            ? candidate
            : null;
    }

    private async Task<string?> ValidateProjectContainerOwnershipAsync(
        Stack stack,
        Domain.Contracts.Resources.PlatformCacheEntry platform,
        string projectName,
        HashSet<string> knownStackContainerIds,
        CancellationToken ct)
    {
        var filter = StackContainerOwnership.CreateComposeProjectContainerFilter(platform.Address, projectName);

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

    private async Task<IReadOnlyCollection<Guid>> LoadRetainedGitSnapshotReleaseIdsAsync(
        Guid stackId,
        Guid currentReleaseId,
        CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var releases = await uow.Stacks.GetReleasesByStackIdAsync(stackId, ct);
        return releases
            .Where(release => release.Id == currentReleaseId || release.IsRollbackCandidate())
            .Select(release => release.Id)
            .ToArray();
    }

    private async Task DiscardFailedGitSnapshotAsync(
        Guid stackId,
        Guid releaseId,
        string? gitSnapshotRoot,
        CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(gitSnapshotRoot))
            return;

        await gitStackMaterializer.DiscardSnapshotAsync(stackId, releaseId, ct);
    }

    private StackApplyCommand BuildApplyCommand(
        Stack stack,
        string platformAddress,
        StackSpec stackSpec,
        string? composeFileContent, string projectName,
        string? environmentFilePath,
        IReadOnlyList<string>? environmentVariables,
        string? registryAuth, string? registryName, string? registryHost,
        IReadOnlyList<string>? serviceNames,
        bool pullImages,
        bool recreate,
        string? sourceWorkingDirectory,
        IReadOnlyList<string>? sourceComposeFilePaths,
        IReadOnlyList<string>? sourceEnvFilePaths,
        string? labelsOverrideFilePath,
        string? generatedFilesDirectory,
        IReadOnlyList<StackSecretFile>? secretFiles,
        IReadOnlyList<string>? secretTargetServiceNames)
        => new(
            PlatformAddress: platformAddress,
            StackName: stack.Name,
            ComposeFileContent: composeFileContent,
            ProjectName: projectName,
            EnvironmentFilePath: environmentFilePath,
            EnvironmentVariables: environmentVariables,
            PreDeploy: stackSpec.PreDeploy,
            PostDeploy: stackSpec.PostDeploy,
            RegistryAuth: registryAuth,
            RegistryName: registryName,
            RegistryHost: registryHost,
            DestroyBeforeDeploy: (recreate || stackSpec.DestroyBeforeDeploy) && serviceNames is not { Count: > 0 },
            Spec: stackSpec,
            ServiceNames: serviceNames,
            PullImages: pullImages,
            SourceWorkingDirectory: sourceWorkingDirectory,
            SourceComposeFilePaths: sourceComposeFilePaths,
            SourceEnvFilePaths: sourceEnvFilePaths,
            LabelsOverrideFilePath: labelsOverrideFilePath,
            GeneratedFilesDirectory: generatedFilesDirectory,
            SecretFiles: secretFiles,
            SecretTargetServiceNames: secretTargetServiceNames);

    private static ResolvedResourceBindings SelectStackApplyConfiguration(
        ResolvedResourceBindings configuration,
        IEnumerable<string> referencedConfigurationKeys)
    {
        var mountedSecretNames = GetMountedFileSecretEntries(configuration).Select(entry => entry.Name);
        return configuration.SelectEntries(referencedConfigurationKeys.Concat(mountedSecretNames));
    }

    private static IReadOnlyList<StackSecretFile> BuildMountedSecretFiles(ResolvedResourceBindings configuration)
        =>
        [
            .. GetMountedFileSecretEntries(configuration)
                .Select(entry => new StackSecretFile(
                    Name: entry.Name,
                    TargetPath: entry.TargetPath!,
                    Content: entry.Value))
        ];

    private static IEnumerable<ResolvedResourceBinding> GetMountedFileSecretEntries(ResolvedResourceBindings configuration)
        => configuration.Entries.Where(entry =>
            entry.Kind == ResourceBindingKind.Secret
            && entry.SecretDeliveryMode == SecretDeliveryMode.MountedFile);

    private Task ProcessConfigurationFailureAlertAsync(Guid stackId, string stackName, string reason, CancellationToken ct)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            StackConfigurationFailures:
            [
                new StackConfigurationResolutionFailureAlertSnapshot(
                    stackId,
                    stackName,
                    reason)
            ]);

        return alertService.ProcessAsync(AlertType.StackConfigurationResolutionFailed, context, ct);
    }

    private static async Task<IReadOnlySet<string>> GetReferencedConfigurationKeysAsync(
        string? composeFileContent,
        IReadOnlyList<string>? sourceComposeFilePaths,
        CancellationToken cancellationToken)
    {
        var references = new HashSet<string>(StringComparer.Ordinal);

        if (!string.IsNullOrWhiteSpace(composeFileContent))
        {
            AddReferencedConfigurationKeys(composeFileContent, references);
        }

        if (sourceComposeFilePaths is { Count: > 0 })
        {
            foreach (var composeFilePath in sourceComposeFilePaths)
            {
                if (!File.Exists(composeFilePath))
                    continue;

                var content = await File.ReadAllTextAsync(composeFilePath, cancellationToken);
                AddReferencedConfigurationKeys(content, references);
            }
        }

        return references;
    }

    private static void AddReferencedConfigurationKeys(string content, HashSet<string> references)
    {
        foreach (Match match in BracedVariableReferenceRegex.Matches(content))
        {
            references.Add(match.Groups["name"].Value);
        }

        foreach (Match match in SimpleVariableReferenceRegex.Matches(content))
        {
            references.Add(match.Groups["name"].Value);
        }
    }

    private static readonly Regex BracedVariableReferenceRegex = new(
        @"(?<!\$)\$\{(?<name>[A-Za-z_][A-Za-z0-9_]*)(?=[:?+\-}]|\})",
        RegexOptions.Compiled);

    private static readonly Regex SimpleVariableReferenceRegex = new(
        @"(?<!\$)\$(?<name>[A-Za-z_][A-Za-z0-9_]*)",
        RegexOptions.Compiled);

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

    private static string GetStackAppliedMessage(StackReleaseStatus? status)
        => status is null or StackReleaseStatus.Healthy
            ? "Stack applied successfully."
            : $"Stack applied with status {status}.";

    private async Task<(bool IsSuccess, Stack? Stack, long? OperationRowVersion, string? ErrorMessage)> MarkProcessingAsync(Guid stackId, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null)
        {
            return (false, null, null, $"Stack with ID {stackId} not found.");
        }

        var previousRelease = stack.CurrentStackRelease;
        var hasRollbackSnapshotForCurrentVersion = previousRelease is not null
            && await HasRollbackSnapshotForCurrentVersionAsync(uow, stack, previousRelease, ct);
        var createNextRelease = ShouldCreateNextReleaseForApply(stack, hasRollbackSnapshotForCurrentVersion);
        var demotePreviousEditedRelease = createNextRelease && hasRollbackSnapshotForCurrentVersion;

        if (!stack.PrepareReleaseForApply(actorId, createNextRelease))
        {
            return (false, null, null, "Stack has no release to apply.");
        }

        if (!stack.MarkProcessing(actorId))
        {
            return (false, null, null, "Stack is already being processed.");
        }

        stack.PartialUpdate(StackReleaseStatus.Applying);
        var claimedReleaseStatus = previousRelease is not null
            && previousRelease.Id != stack.CurrentStackReleaseId
                ? previousRelease.Status
                : stack.CurrentStackRelease?.Status ?? StackReleaseStatus.Applying;
        var operationRowVersion = stack.RowVersion + 1;
        var claimed = await uow.Stacks.UpdateProcessingAsync(
            stack.Id,
            claimedReleaseStatus,
            stack.ControlState,
            stack.ControlStartedAt,
            stack.RowVersion,
            checkRowVersion: true,
            stack.ControlTriggeredBy,
            ct);
        if (!claimed)
        {
            return (false, null, null, "Stack is already being processed.");
        }

        if (demotePreviousEditedRelease && previousRelease is not null && previousRelease.Id != stack.CurrentStackReleaseId)
        {
            await uow.Stacks.UpdateReleaseStatusAsync(previousRelease.Id, StackReleaseStatus.Created, ct);
        }

        await uow.Stacks.UpdateAsync(stack, ct);
        await uow.CommitAsync(ct);
        return (true, stack, operationRowVersion, null);
    }

    private static async Task<bool> HasRollbackSnapshotForCurrentVersionAsync(
        IUnitOfWork uow,
        Stack stack,
        StackRelease currentRelease,
        CancellationToken ct)
    {
        var releases = await uow.Stacks.GetReleasesByStackIdAsync(stack.Id, ct);
        return releases?.Any(release =>
            release.Id != currentRelease.Id &&
            release.Version == currentRelease.Version &&
            release.IsRollbackCandidate()) == true;
    }

    private static bool ShouldCreateNextReleaseForApply(Stack stack, bool hasRollbackSnapshotForCurrentVersion)
    {
        if (stack.CurrentStackRelease?.Status is StackReleaseStatus.Created or StackReleaseStatus.Failed)
            return false;

        if (hasRollbackSnapshotForCurrentVersion)
            return true;

        if (stack.CurrentStackRelease?.Spec is GitStack
            && stack.StackUpdateState is GitStackUpdateState gitState)
        {
            var commitState = gitState.RecreateStackOnNewCommitState;
            return !string.IsNullOrWhiteSpace(commitState.RemoteCommitSha)
                && !string.Equals(commitState.CurrentCommitSha, commitState.RemoteCommitSha, StringComparison.OrdinalIgnoreCase);
        }

        return false;
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

    private async Task<GitRepository?> LoadGitRepository(Guid gitRepositoryId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.GitRepositories.GetWithAccountAsync(gitRepositoryId, ct);
    }

    private static IReadOnlyList<AppliedStackBuildImage> ResolveAppliedBuildImages(
        IReadOnlyList<ResolvedStackBuildImageBinding> resolvedBindings,
        IReadOnlyList<string>? requestedServiceNames,
        IReadOnlyList<string>? availableServiceNames)
    {
        var requested = requestedServiceNames is { Count: > 0 }
            ? requestedServiceNames.ToHashSet(StringComparer.OrdinalIgnoreCase)
            : null;
        var available = availableServiceNames is { Count: > 0 }
            ? availableServiceNames.ToHashSet(StringComparer.OrdinalIgnoreCase)
            : null;

        return resolvedBindings
            .Where(binding => requested is null || requested.Contains(binding.Binding.ServiceName))
            .Where(binding => available is null || available.Contains(binding.Binding.ServiceName))
            .Select(binding => new AppliedStackBuildImage(
                binding.Binding.ServiceName,
                binding.Binding.BuildProjectId,
                binding.ImageReference,
                binding.Digest,
                binding.BuildRunId))
            .ToArray();
    }

    private ValueTask EnqueueStatus(
        Guid stackId,
        Guid actorId,
        StackReleaseStatus status,
        string? message,
        StackApplyOperation operation,
        StackSnapshot? previousStackSnapshot = null,
        StackReleaseSource? source = null,
        IReadOnlyList<ResourceBindingSnapshot>? resourceBindings = null,
        long? expectedRowVersion = null,
        CancellationToken ct = default)
        => dbWorkQueue.EnqueueAsync(
            new UpdateStackStatusWorkItem(
                stackId,
                actorId,
                status,
                message,
                stackHub,
                activityHub,
                notificationQueue,
                operation,
                previousStackSnapshot,
                source,
                resourceBindings,
                expectedRowVersion),
            ct);
}

internal sealed class UpdateStackStatusWorkItem(Guid stackId, Guid actorId, StackReleaseStatus status, string? message, IStackStreamManager stackHub,
    IActivityStreamManager activityHub, INotificationQueue notificationQueue, StackApplyOperation operation, StackSnapshot? previousStackSnapshot = null, StackReleaseSource? source = null, IReadOnlyList<ResourceBindingSnapshot>? resourceBindings = null, long? expectedRowVersion = null) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null) return;

        if (!await StackOperationOwnership.TryCompleteAsync(uow, stack, status, expectedRowVersion, ct))
            return;

        if (source is not null)
        {
            stack.CurrentStackRelease?.UpdateSource(source);
        }

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
                            eventType: StackActivityFactory.GetEventType(operation),
                            info: StackActivityFactory.CreateInfo(
                                operation,
                                previousStackSnapshot,
                                stack.ToSnapshot(),
                                new StackResultSnapshot(null, Helpers.RemoveAnsiSequences(message ?? ""), resourceBindings))
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
    StackReleaseStatus? composeStatus,
    IStackStreamManager stackHub,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    StackApplyOperation operation,
    StackSnapshot? previousStackSnapshot = null,
    StackReleaseSource? source = null,
    IReadOnlyList<ResourceBindingSnapshot>? resourceBindings = null,
    IReadOnlyList<StackReleaseVolumeBinding>? volumeBindings = null,
    IReadOnlyList<AppliedStackBuildImage>? appliedBuildImages = null,
    long? operationRowVersion = null) : IDbWorkItem
{
    public bool Applied { get; private set; }

    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack?.CurrentStackRelease is null) return;

        var completionStatus = composeStatus ?? GetAppliedStackStatus(dockerContainers);
        if (!await StackOperationOwnership.TryCompleteAsync(uow, stack, completionStatus, operationRowVersion, ct))
            return;

        if (source is not null)
        {
            stack.CurrentStackRelease.UpdateSource(source);

            if (stack.StackUpdateState is GitStackUpdateState gitState)
            {
                stack.SetStackUpdateState(gitState with
                {
                    RecreateStackOnNewCommitState = gitState.RecreateStackOnNewCommitState with
                    {
                        CurrentCommitSha = source.ResolvedCommitSha,
                        RemoteCommitSha = null,
                        LastCheckedAt = DateTime.UtcNow
                    }
                });
            }
        }

        stack.CurrentStackRelease.UpdateResourceBindings(resourceBindings);
        UpdateAppliedBuildImages(stack.CurrentStackRelease, appliedBuildImages);

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
        await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
            stack.CurrentStackRelease.Id,
            DeduplicateVolumeBindings(volumeBindings),
            ct);
        await uow.Stacks.UpdateAsync(stack, ct);

        var containerIds = upserts.Select(container => container.DockerContainerId).ToArray();
        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: stack.Id,
                        platformId: stack.CurrentStackRelease?.PlatformId,
                        resourceName: stack.Name,
                        status: ActivityStatus.Success,
                        eventType: StackActivityFactory.GetEventType(operation),
                        info: StackActivityFactory.CreateInfo(
                            operation,
                            previousStackSnapshot,
                            stack.ToSnapshot(),
                            new StackResultSnapshot(containerIds, operation == StackApplyOperation.Rollback ? "Stack rolled back successfully." : "Stack applied successfully.", resourceBindings))
                        );

        await uow.ActivityEventRepository.AddAsync(activity, ct);
        stack.AssignActivityEvent(activity);

        await uow.CommitAsync(ct);
        Applied = true;
        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, stack), ct);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct)), ct);
    }

    private static StackReleaseStatus GetAppliedStackStatus(IReadOnlyCollection<DockerContainer> containers)
        => containers.Count == 0
            ? StackReleaseStatus.Healthy
            : Stack.ToStackStatus(containers.Select(container => container.State));

    private static void UpdateAppliedBuildImages(
        StackRelease release,
        IReadOnlyList<AppliedStackBuildImage>? appliedImages)
    {
        if (appliedImages is not { Count: > 0 } || release.Spec.BuildImageBindings is not { Count: > 0 })
            return;

        var appliedAt = DateTimeOffset.UtcNow;
        var bindings = release.Spec.BuildImageBindings.Select(binding =>
        {
            var applied = appliedImages.FirstOrDefault(candidate =>
                candidate.BuildProjectId == binding.BuildProjectId
                && string.Equals(candidate.ServiceName, binding.ServiceName, StringComparison.OrdinalIgnoreCase));
            if (applied is null)
                return binding;

            var hasResolvedArtifact = !string.IsNullOrWhiteSpace(binding.ResolvedImageReference);
            return binding with
            {
                ResolvedImageReference = hasResolvedArtifact
                    ? binding.ResolvedImageReference
                    : applied.ImageReference,
                ResolvedDigest = hasResolvedArtifact
                    ? binding.ResolvedDigest
                    : applied.Digest,
                ResolvedBuildRunId = hasResolvedArtifact
                    ? binding.ResolvedBuildRunId
                    : applied.BuildRunId,
                AppliedImageReference = applied.ImageReference,
                AppliedDigest = applied.Digest,
                AppliedBuildRunId = applied.BuildRunId,
                AppliedAt = appliedAt
            };
        }).ToArray();

        release.UpdateSpec(release.Spec.WithBuildImageBindings(bindings));
    }

    private static IReadOnlyList<StackReleaseVolumeBinding> DeduplicateVolumeBindings(
        IReadOnlyList<StackReleaseVolumeBinding>? volumeBindings)
    {
        if (volumeBindings is null or { Count: 0 })
            return [];

        var bindings = new Dictionary<string, StackReleaseVolumeBinding>(StringComparer.Ordinal);
        foreach (var binding in volumeBindings)
        {
            bindings.TryAdd(binding.VolumeName, binding);
        }

        return [.. bindings.Values];
    }
}

internal static class StackOperationOwnership
{
    public static async Task<bool> TryCompleteAsync(
        IUnitOfWork uow,
        Stack stack,
        StackReleaseStatus status,
        long? expectedRowVersion,
        CancellationToken ct)
    {
        if (expectedRowVersion.HasValue)
        {
            var completed = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                status,
                ResourceControlState.Idle,
                startedAt: null,
                expectedRowVersion.Value,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);
            if (!completed)
                return false;
        }

        return stack.ReleaseProcessing(status);
    }
}

internal sealed record AppliedStackBuildImage(
    string ServiceName,
    Guid BuildProjectId,
    string ImageReference,
    string? Digest,
    Guid? BuildRunId);

internal static class StackActivityFactory
{
    public static ActivityEventType GetEventType(StackApplyOperation operation)
        => operation == StackApplyOperation.Rollback
            ? ActivityEventType.StackRollback
            : ActivityEventType.StackApplied;

    public static ActivityEventInfo CreateInfo(
        StackApplyOperation operation,
        StackSnapshot? previousStackSnapshot,
        StackSnapshot? stack,
        StackResultSnapshot result)
        => operation == StackApplyOperation.Rollback
            ? new StackRollback(previousStackSnapshot, stack, result)
            : new StackApplied(stack, result);
}

internal sealed class StackNotificationWorkItem(IStackStreamManager stackHub, Stack stack, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => stackHub.SendStackInfo(stack, action);
}
