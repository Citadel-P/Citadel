using Application.Features.Deployments.Notifications;
using Application.Features.Stacks;
using Application.Configs;
using Application.Services.Builds;
using Application.Mappers;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Contracts.Resources.Swarm;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
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
    IStackOperationBarrier? operationBarrier = null,
    IConnectorFactory<ISwarmConnector>? swarmConnectorFactory = null,
    ISwarmReconciliationCoordinator? swarmReconciliationCoordinator = null,
    IConnectorFactory<IVolumeConnector>? volumeConnectorFactory = null,
    IOptions<SwarmStackOptions>? swarmStackOptions = null) : IApplyStackService
{
    private const int MaxTransportSourceFiles = 512;
    private const long MaxTransportSourceBytes = 12L * 1024 * 1024;
    private readonly IStackOperationBarrier _operationBarrier = operationBarrier ?? new NoOpStackOperationBarrier();
    private readonly int _retainedSwarmRollbackReleases = swarmStackOptions?.Value.RetainedRollbackReleases ?? 10;

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

        var isSwarmStack = stack.CurrentStackRelease?.Platform?.PlatformDescriptor.Type == PlatformType.DockerSwarm;
        var convertComposeProjectToSwarm = isSwarmStack
            && operation == StackApplyOperation.Apply
            && (await LoadStackContainersAsync(stack.Id, ct)).Any(static container => !container.IsSwarmTask);

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

        var currentRelease = stack.CurrentStackRelease!;

        if (isSwarmStack
            && (currentRelease.Platform?.PlatformDescriptor is not Domain.Entities.Platforms.DockerSwarmPlatformDescriptor swarmDescriptor
                || !swarmDescriptor.ControlAvailable
                || !string.Equals(swarmDescriptor.LocalNodeState, "active", StringComparison.OrdinalIgnoreCase)))
        {
            var message = "Swarm Stack apply requires a connected Swarm manager with control available.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

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

        if (!isSwarmStack)
        {
            var knownStackContainerIds = await LoadStackContainerIds(stack.Id, ct);
            var collisionMessage = await ValidateProjectContainerOwnershipAsync(stack, platform, projectName, knownStackContainerIds, ct);
            if (!string.IsNullOrWhiteSpace(collisionMessage))
            {
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, collisionMessage, operation: operation, expectedRowVersion: stack.RowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(collisionMessage, 1);
                yield break;
            }
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
        var retainedSwarmResources = isSwarmStack && operation == StackApplyOperation.Rollback
            ? await LoadReleaseSwarmResourcesAsync(currentRelease.Id, ct)
            : [];
        var retainedSwarmSecrets = retainedSwarmResources
            .Where(static resource => resource.Kind == StackReleaseSwarmResourceKind.Secret)
            .ToArray();
        var retainedMountedSecrets = retainedSwarmSecrets
            .Where(static resource => resource.Mounts.Count > 0
                                      && resource.ComposeResourceName.StartsWith("citadel-", StringComparison.Ordinal))
            .ToArray();
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
        string? sourceTransportRoot = null;
        IReadOnlyList<string>? secretTargetServiceNames = null;
        StackReleaseSource? releaseSource = null;
        IReadOnlyList<string> swarmPreflightComposeFiles = [];
        yield return StackStreamItem.SystemMessage("Resolving stack variables and secrets...", 0);
        var configurationResult = retainedMountedSecrets.Length == 0
            ? await resourceBinderResolver.ResolveAsync(ResourceBindingScope.Stack, stack.Id, ct)
            : await resourceBinderResolver.ResolveWithoutMountedSecretsAsync(ResourceBindingScope.Stack, stack.Id, ct);
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
            swarmPreflightComposeFiles = isSwarmStack ? [composeWithBuildImages] : [];
            composeFileContent = isSwarmStack
                ? StackComposeLabelInjector.InjectSwarm(composeWithBuildImages, stack.Id, currentRelease.Id)
                : StackComposeLabelInjector.Inject(composeWithBuildImages, stack.Id, currentRelease.Id);
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
            if (isSwarmStack)
            {
                swarmPreflightComposeFiles = await ReadComposeFilesAsync(sourceComposeFilePaths, ct);
            }
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
            sourceTransportRoot = Path.GetDirectoryName(payload.SnapshotRoot);
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
        var secretFiles = retainedMountedSecrets.Length > 0
            ? []
            : BuildMountedSecretFiles(selectedConfiguration);
        if (isSwarmStack && secretFiles.Any(static secret => !IsSwarmSecretTarget(secret.TargetPath)))
        {
            var message = "Swarm Stack secret targets must be files under /run/secrets/.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
            yield return StackStreamItem.FromStdErr(message, 1);
            yield break;
        }

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

        IReadOnlySet<string> expectedSwarmServices = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        if (isSwarmStack)
        {
            if (serviceNames is { Count: > 0 } || recreate)
            {
                var message = "Service-scoped apply and recreate are not supported for Swarm Stacks.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            var policyIssues = SwarmStackConfigurationPolicy.GetIssues(stackSpec, stack.DriftPolicy);
            var compatibility = StackComposeParser.AnalyzeSwarmCompatibility(
                swarmPreflightComposeFiles,
                stackSpec.BuildImageBindings);
            var errors = policyIssues.Concat(compatibility.Issues)
                .Where(static issue => issue.Severity == SwarmStackCompatibilitySeverity.Error)
                .ToArray();
            if (errors.Length > 0)
            {
                var message = string.Join(Environment.NewLine, errors.Select(static issue => issue.Message));
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            foreach (var warning in compatibility.Issues.Where(static issue => issue.Severity == SwarmStackCompatibilitySeverity.Warning))
                yield return StackStreamItem.SystemMessage($"Preflight warning: {warning.Message}", 0);

            expectedSwarmServices = swarmPreflightComposeFiles
                .SelectMany(content => StackComposeParser.ParseServices(stack.Id, currentRelease.Id, content).Keys)
                .ToHashSet(StringComparer.OrdinalIgnoreCase);

            if (swarmConnectorFactory is null)
            {
                var message = "Swarm connector is not configured.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            var externalResources = StackComposeParser.ParseSwarmExternalResources(swarmPreflightComposeFiles);
            var swarmConnector = swarmConnectorFactory.GetConnector(platform.ConnectorType);
            var volumeConnector = externalResources.Volumes.Count == 0
                ? null
                : volumeConnectorFactory?.GetConnector(platform.ConnectorType);
            var linkedServices = await LoadSwarmLinkedServicesAsync(
                stack.Id,
                currentRelease.PlatformId,
                projectName,
                ct);

            if (!await TryReserveSwarmNamespaceAsync(
                    stack.Id,
                    currentRelease.PlatformId,
                    projectName,
                    ct))
            {
                var message = $"Swarm Stack namespace '{projectName}' is already reserved by another Stack, or this Stack is already bound to a different Swarm namespace.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            var collisionMessage = await ValidateSwarmNamespaceOwnershipAsync(
                swarmConnector,
                volumeConnector,
                platform.Address,
                projectName,
                stack.Id,
                linkedServices.ServiceIds,
                linkedServices.ReclaimableStackIds,
                externalResources,
                retainedSwarmResources,
                ct);
            if (collisionMessage is not null)
            {
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, collisionMessage, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(collisionMessage, 1);
                yield break;
            }

            yield return StackStreamItem.SystemMessage(
                $"Swarm preflight passed for namespace '{projectName}' ({expectedSwarmServices.Count} service{(expectedSwarmServices.Count == 1 ? string.Empty : "s")}).",
                0);
        }

        IReadOnlyList<StackSourceFile>? transportSourceFiles = null;
        if (isSwarmStack
            && platform.ConnectorType is PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent
            && !string.IsNullOrWhiteSpace(sourceTransportRoot))
        {
            var transportSource = await CreateTransportSourceBundleAsync(sourceTransportRoot, ct);
            if (transportSource.ErrorMessage is not null)
            {
                await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, transportSource.ErrorMessage, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                yield return StackStreamItem.FromStdErr(transportSource.ErrorMessage, 1);
                yield break;
            }

            transportSourceFiles = transportSource.Files;
            sourceWorkingDirectory = ToTransportRelativePath(sourceTransportRoot, sourceWorkingDirectory!);
            sourceComposeFilePaths = sourceComposeFilePaths?
                .Select(path => ToTransportRelativePath(sourceTransportRoot, path))
                .ToArray();
            sourceEnvFilePaths = sourceEnvFilePaths?
                .Select(path => ToTransportRelativePath(sourceTransportRoot, path))
                .ToArray();
            labelsOverrideFilePath = string.IsNullOrWhiteSpace(labelsOverrideFilePath)
                ? null
                : ToTransportRelativePath(sourceTransportRoot, labelsOverrideFilePath);
            generatedFilesDirectory = string.IsNullOrWhiteSpace(generatedFilesDirectory)
                ? null
                : ToTransportRelativePath(sourceTransportRoot, generatedFilesDirectory);
        }

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
            secretTargetServiceNames,
            isSwarmStack,
            transportSourceFiles,
            [.. retainedSwarmSecrets.Select(static resource => new StackRetainedSwarmSecret(
                resource.ComposeResourceName,
                resource.DockerResourceName,
                resource.Mounts))],
            [.. retainedSwarmResources
                .Where(static resource => resource.Kind == StackReleaseSwarmResourceKind.Config)
                .Select(static resource => new StackRetainedSwarmConfig(
                    resource.ComposeResourceName,
                    resource.DockerResourceName,
                    resource.Mounts))],
            convertComposeProjectToSwarm);

        int? exitCode = null;
        StackReleaseStatus? composeStatus = null;
        string? lastErrorLog = null;
        var enumerator = connector.StackApplyAsync(command, ct).GetAsyncEnumerator(ct);

        try
        {
            while (true)
            {
                var next = await TryReadNextAsync(enumerator);
                if (next.ErrorMessage is not null)
                {
                    var safeError = secretRedactor.Redact(next.ErrorMessage, selectedConfiguration.RedactionValues);
                    if (isSwarmStack && ct.IsCancellationRequested)
                    {
                        safeError = "Stack deployment was interrupted after dispatch. Citadel will reconcile the Swarm state in the background.";
                        using var persistenceTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
                        await EnqueueStatus(
                            stack.Id,
                            actorId,
                            StackReleaseStatus.Unknown,
                            safeError,
                            operation: operation,
                            source: releaseSource,
                            resourceBindings: selectedConfiguration.SnapshotEntries,
                            expectedRowVersion: operationRowVersion,
                            ct: persistenceTimeout.Token);
                    }
                    else
                    {
                        await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, safeError, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                    }

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
                            safeMessage.Contains("failed", StringComparison.OrdinalIgnoreCase) ||
                            safeMessage.Contains("timed out", StringComparison.OrdinalIgnoreCase))
                        {
                            lastErrorLog = safeMessage.Trim();
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

                    if (result.ExitCode != 0 && !isSwarmStack)
                    {
                        var explicitFailure = lastErrorLog
                            ?? $"Pipeline command failed with exit code {result.ExitCode}.";

                        await DiscardFailedGitSnapshotAsync(stack.Id, currentRelease.Id, gitSnapshotRoot, ct);
                        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, explicitFailure, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: ct);
                        yield return StackStreamItem.FromStdErr(explicitFailure, result.ExitCode.Value);
                        yield break;
                    }

                    yield return StackStreamItem.Finished(result.ExitCode.Value);
                }
            }
        }
        finally
        {
            await enumerator.DisposeAsync();
        }

        if (isSwarmStack)
        {
            if (exitCode != 0)
            {
                var status = exitCode == 124 ? StackReleaseStatus.TimedOut : StackReleaseStatus.Failed;
                var message = lastErrorLog
                    ?? (exitCode is int stackExitCode
                        ? $"docker stack deploy exited with code {stackExitCode}."
                        : "docker stack deploy did not report a completion exit code.");
                using var failurePersistenceTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
                if (status == StackReleaseStatus.Failed)
                {
                    await DiscardFailedGitSnapshotAsync(
                        stack.Id,
                        currentRelease.Id,
                        gitSnapshotRoot,
                        failurePersistenceTimeout.Token);
                }

                await EnqueueStatus(stack.Id, actorId, status, message, operation: operation, source: releaseSource, resourceBindings: selectedConfiguration.SnapshotEntries, expectedRowVersion: operationRowVersion, ct: failurePersistenceTimeout.Token);
                yield return StackStreamItem.FromStdErr(message, exitCode ?? 1);
                yield break;
            }

            yield return StackStreamItem.SystemMessage(
                "Docker accepted the Stack definition. Waiting for Swarm Services and Tasks to converge...",
                0);

            if (swarmReconciliationCoordinator is not null)
            {
                var refresh = await swarmReconciliationCoordinator.RefreshAsync(platform.Id, CancellationToken.None);
                if (refresh.IsFailure())
                {
                    yield return StackStreamItem.SystemMessage(
                        "The immediate Swarm inventory refresh did not complete. Docker events and background reconciliation will retry it.",
                        0);
                }
            }

            using var convergenceTimeout = new CancellationTokenSource(TimeSpan.FromMinutes(2));
            var convergence = await WaitForSwarmConvergenceAsync(
                swarmConnectorFactory!.GetConnector(platform.ConnectorType),
                platform.Address,
                projectName,
                stack.Id,
                currentRelease.Id,
                expectedSwarmServices,
                convergenceTimeout.Token);
            using var persistenceTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
            var completionToken = persistenceTimeout.Token;

            if (convergence.Status != StackReleaseStatus.Healthy)
            {
                await EnqueueStatus(
                    stack.Id,
                    actorId,
                    convergence.Status,
                    convergence.Message,
                    operation: operation,
                    source: releaseSource,
                    resourceBindings: selectedConfiguration.SnapshotEntries,
                    expectedRowVersion: operationRowVersion,
                    ct: completionToken);
                yield return StackStreamItem.FromStdErr(convergence.Message, 1);
                yield break;
            }

            var capturedResources = await CaptureSwarmReleaseResourcesAsync(
                swarmConnectorFactory!.GetConnector(platform.ConnectorType),
                platform.Address,
                platform.Id,
                projectName,
                currentRelease.Id,
                convergence.Services,
                completionToken);
            if (capturedResources.ErrorMessage is not null)
            {
                var message = $"The Stack converged, but its immutable Swarm resources could not be recorded: {capturedResources.ErrorMessage}";
                await EnqueueStatus(
                    stack.Id,
                    actorId,
                    StackReleaseStatus.Unknown,
                    message,
                    operation: operation,
                    source: releaseSource,
                    resourceBindings: selectedConfiguration.SnapshotEntries,
                    expectedRowVersion: operationRowVersion,
                    ct: completionToken);
                yield return StackStreamItem.FromStdErr(message, 1);
                yield break;
            }

            if (!string.IsNullOrWhiteSpace(gitSnapshotRoot))
            {
                await gitStackMaterializer.ActivateCurrentAsync(stack.Id, gitSnapshotRoot, completionToken);
                var retainedReleaseIds = await LoadRetainedGitSnapshotReleaseIdsAsync(
                    stack.Id,
                    stack.CurrentStackReleaseId,
                    completionToken);
                await gitStackMaterializer.PruneSnapshotsAsync(stack.Id, retainedReleaseIds, completionToken);
            }

            await _operationBarrier.WaitAsync(
                StackOperationCheckpoint.ExternalStateCaptured,
                stack.Id,
                operationRowVersion,
                completionToken);

            var workItem = new StackSucceededWorkItem(
                stack.Id,
                actorId,
                [],
                StackReleaseStatus.Healthy,
                stackHub,
                activityHub,
                notificationQueue,
                operation,
                previousStackSnapshot,
                releaseSource,
                selectedConfiguration.SnapshotEntries,
                volumeBindings: [],
                ResolveAppliedBuildImages(
                    resolvedBuildBindings.Bindings,
                    serviceNames,
                    secretTargetServiceNames),
                operationRowVersion,
                capturedResources.Resources);

            await dbWorkQueue.EnqueueAndWaitAsync(workItem, completionToken);
            if (!workItem.Applied)
            {
                yield return StackStreamItem.FromStdErr(
                    "Stack operation was superseded before its result could be committed.",
                    1);
                yield break;
            }

            var cleanupWarning = await TryCleanupOldSwarmReleaseResourcesAsync(
                swarmConnectorFactory.GetConnector(platform.ConnectorType),
                platform.Address,
                stack.Id,
                currentRelease.Id,
                capturedResources.Resources,
                selectedConfiguration.RedactionValues,
                completionToken);
            if (cleanupWarning is not null)
                yield return StackStreamItem.SystemMessage(cleanupWarning, 0);

            yield return StackStreamItem.SystemMessage(
                "Swarm Stack converged successfully.",
                0,
                StackReleaseStatus.Healthy);
            yield break;
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

        var finalFailureMessage = lastErrorLog
            ?? (exitCode is int code ? $"docker compose exited with code {code}." : "Stack apply did not report a completion exit code.");

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

    private async Task<IReadOnlyList<Container>> LoadStackContainersAsync(Guid stackId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return [.. await uow.Stacks.GetContainersAsync(stackId, ct) ?? []];
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
        IReadOnlyList<string>? secretTargetServiceNames,
        bool isSwarmStack,
        IReadOnlyList<StackSourceFile>? sourceFiles,
        IReadOnlyList<StackRetainedSwarmSecret>? retainedSwarmSecrets,
        IReadOnlyList<StackRetainedSwarmConfig>? retainedSwarmConfigs,
        bool convertComposeProjectToSwarm)
        => new(
            PlatformAddress: platformAddress,
            StackName: stack.Name,
            ComposeFileContent: composeFileContent,
            ProjectName: projectName,
            EnvironmentFilePath: isSwarmStack ? null : environmentFilePath,
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
            SecretTargetServiceNames: secretTargetServiceNames,
            OrchestrationMode: isSwarmStack
                ? StackOrchestrationMode.DockerSwarm
                : StackOrchestrationMode.DockerCompose,
            SourceFiles: sourceFiles,
            RetainedSwarmSecrets: retainedSwarmSecrets,
            RetainedSwarmConfigs: retainedSwarmConfigs,
            ConvertComposeProjectToSwarm: convertComposeProjectToSwarm);

    private static async Task<TransportSourceBundleResult> CreateTransportSourceBundleAsync(
        string rootPath,
        CancellationToken cancellationToken)
    {
        var root = Path.GetFullPath(rootPath);
        if (!Directory.Exists(root))
            return TransportSourceBundleResult.Failure("The materialized Git Stack source is no longer available.");

        var files = new List<StackSourceFile>();
        long totalBytes = 0;
        var enumerationOptions = new EnumerationOptions
        {
            RecurseSubdirectories = true,
            AttributesToSkip = FileAttributes.ReparsePoint,
            IgnoreInaccessible = false
        };

        try
        {
            foreach (var path in Directory.EnumerateFiles(root, "*", enumerationOptions))
            {
                cancellationToken.ThrowIfCancellationRequested();
                if (files.Count >= MaxTransportSourceFiles)
                {
                    return TransportSourceBundleResult.Failure(
                        $"Git Stack source contains more than {MaxTransportSourceFiles} files required for remote deployment.");
                }

                var fileLength = new FileInfo(path).Length;
                if (fileLength > MaxTransportSourceBytes - totalBytes)
                {
                    return TransportSourceBundleResult.Failure(
                        $"Git Stack source exceeds the {MaxTransportSourceBytes / (1024 * 1024)} MiB remote deployment limit.");
                }

                files.Add(new StackSourceFile(
                    ToTransportRelativePath(root, path),
                    await File.ReadAllBytesAsync(path, cancellationToken)));
                totalBytes += fileLength;
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            return TransportSourceBundleResult.Failure($"Unable to package Git Stack source: {ex.Message}");
        }

        return new(files, null);
    }

    private static string ToTransportRelativePath(string rootPath, string path)
    {
        var root = Path.GetFullPath(rootPath);
        var fullPath = Path.GetFullPath(path);
        var relativePath = Path.GetRelativePath(root, fullPath);
        if (Path.IsPathRooted(relativePath)
            || relativePath.Equals("..", StringComparison.Ordinal)
            || relativePath.StartsWith($"..{Path.DirectorySeparatorChar}", StringComparison.Ordinal))
        {
            throw new InvalidDataException("Git Stack source path escapes its immutable release directory.");
        }

        return relativePath.Replace('\\', '/');
    }

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

    private static bool IsSwarmSecretTarget(string targetPath)
    {
        const string secretRoot = "/run/secrets/";
        return targetPath.StartsWith(secretRoot, StringComparison.Ordinal)
            && targetPath.Length > secretRoot.Length
            && !targetPath[secretRoot.Length..].Contains('/');
    }

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

    private static async Task<IReadOnlyList<string>> ReadComposeFilesAsync(
        IReadOnlyList<string>? composeFilePaths,
        CancellationToken cancellationToken)
    {
        if (composeFilePaths is not { Count: > 0 })
            return [];

        var contents = new List<string>(composeFilePaths.Count);
        foreach (var path in composeFilePaths)
            contents.Add(await File.ReadAllTextAsync(path, cancellationToken));
        return contents;
    }

    private static async Task<string?> ValidateSwarmNamespaceOwnershipAsync(
        ISwarmConnector connector,
        IVolumeConnector? volumeConnector,
        string platformAddress,
        string stackNamespace,
        Guid stackId,
        IReadOnlySet<string> linkedServiceIds,
        IReadOnlySet<string> reclaimableStackIds,
        SwarmExternalResourceReferences externalResources,
        IReadOnlyList<StackReleaseSwarmResource> retainedResources,
        CancellationToken cancellationToken)
    {
        var servicesTask = connector.ListServicesAsync(
            new ListSwarmServicesCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var networksTask = connector.ListNetworksAsync(
            new ListSwarmNetworksCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var secretsTask = connector.ListSecretsAsync(
            new ListSwarmSecretsCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var configsTask = connector.ListConfigsAsync(
            new ListSwarmConfigsCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);

        await Task.WhenAll(servicesTask, networksTask, secretsTask, configsTask);

        var servicesResult = await servicesTask;
        if (servicesResult.IsFailure(out var serviceError, out var services))
            return $"Unable to validate Swarm Stack Service ownership: {serviceError.Message}";
        var networksResult = await networksTask;
        if (networksResult.IsFailure(out var networkError, out var networks))
            return $"Unable to validate Swarm Stack Network ownership: {networkError.Message}";
        var secretsResult = await secretsTask;
        if (secretsResult.IsFailure(out var secretError, out var secrets))
            return $"Unable to validate Swarm Stack Secret ownership: {secretError.Message}";
        var configsResult = await configsTask;
        if (configsResult.IsFailure(out var configError, out var configs))
            return $"Unable to validate Swarm Stack Config ownership: {configError.Message}";

        foreach (var retained in retainedResources)
        {
            var exists = retained.Kind switch
            {
                StackReleaseSwarmResourceKind.Secret => secrets.Any(resource =>
                    string.Equals(resource.Id, retained.DockerResourceId, StringComparison.Ordinal)
                    && string.Equals(resource.Name, retained.DockerResourceName, StringComparison.Ordinal)),
                StackReleaseSwarmResourceKind.Config => configs.Any(resource =>
                    string.Equals(resource.Id, retained.DockerResourceId, StringComparison.Ordinal)
                    && string.Equals(resource.Name, retained.DockerResourceName, StringComparison.Ordinal)),
                _ => false
            };
            if (!exists)
            {
                return $"Rollback requires retained Swarm {retained.Kind} '{retained.DockerResourceName}' ({retained.DockerResourceId}), but it is no longer available.";
            }
        }

        var missingNetwork = externalResources.Networks.FirstOrDefault(name =>
            !networks.Any(network => string.Equals(network.Name, name, StringComparison.Ordinal)));
        if (missingNetwork is not null)
            return $"External Swarm Network '{missingNetwork}' does not exist on the selected platform.";

        var missingSecret = externalResources.Secrets.FirstOrDefault(name =>
            !secrets.Any(secret => string.Equals(secret.Name, name, StringComparison.Ordinal)));
        if (missingSecret is not null)
            return $"External Swarm Secret '{missingSecret}' does not exist on the selected platform.";

        var missingConfig = externalResources.Configs.FirstOrDefault(name =>
            !configs.Any(config => string.Equals(config.Name, name, StringComparison.Ordinal)));
        if (missingConfig is not null)
            return $"External Swarm Config '{missingConfig}' does not exist on the selected platform.";

        if (externalResources.Volumes.Count > 0)
        {
            if (volumeConnector is null)
                return "External Swarm Volume validation is not available for this connector.";

            var volumesResult = await volumeConnector.ListVolumesAsync(
                new ListdDockerVolumesCommand(platformAddress, null, null, null),
                cancellationToken);
            if (volumesResult.IsFailure(out var volumeError, out var volumes))
                return $"Unable to validate external Swarm Volumes: {volumeError.Message}";

            var volumeNames = volumes.Select(static volume => volume.Name).ToHashSet(StringComparer.Ordinal);
            var missingVolume = externalResources.Volumes.FirstOrDefault(name => !volumeNames.Contains(name));
            if (missingVolume is not null)
                return $"External Swarm Volume '{missingVolume}' does not exist on the selected platform.";
        }

        var expectedStackId = stackId.ToString("D");
        var ownedServices = services.Where(service =>
                HasLabel(service.Labels, "com.docker.stack.namespace", stackNamespace)
                && (HasStackOwnership(service.Labels, expectedStackId)
                    || linkedServiceIds.Contains(service.Id)
                    && (!HasAnyCitadelOwnershipLabel(service.Labels)
                        || HasReclaimableStackOwnership(service.Labels, reclaimableStackIds))))
            .ToArray();
        var serviceConflict = FindSwarmNamespaceOwnershipConflict(
            services.Select(static service => (service.Id, service.Name, service.Labels)),
            "Service",
            stackNamespace,
            expectedStackId,
            linkedServiceIds,
            reclaimableStackIds);
        if (serviceConflict is not null)
            return serviceConflict;

        var networkConflict = FindSwarmNamespaceOwnershipConflict(
            networks.Select(static network => (network.Id, network.Name, network.Labels)),
            "Network",
            stackNamespace,
            expectedStackId,
            ownedServices.SelectMany(static service => service.NetworkIds).ToHashSet(StringComparer.Ordinal),
            reclaimableStackIds);
        if (networkConflict is not null)
            return networkConflict;

        var secretConflict = FindSwarmNamespaceOwnershipConflict(
            secrets.Select(static secret => (secret.Id, secret.Name, secret.Labels)),
            "Secret",
            stackNamespace,
            expectedStackId,
            ownedServices.SelectMany(static service => service.SecretIds).ToHashSet(StringComparer.Ordinal),
            reclaimableStackIds);
        if (secretConflict is not null)
            return secretConflict;

        return FindSwarmNamespaceOwnershipConflict(
            configs.Select(static config => (config.Id, config.Name, config.Labels)),
            "Config",
            stackNamespace,
            expectedStackId,
            ownedServices.SelectMany(static service => service.ConfigIds).ToHashSet(StringComparer.Ordinal),
            reclaimableStackIds);
    }

    private static string? FindSwarmNamespaceOwnershipConflict(
        IEnumerable<(string Id, string Name, IReadOnlyDictionary<string, string> Labels)> resources,
        string resourceType,
        string stackNamespace,
        string expectedStackId,
        IReadOnlySet<string> resourcesReferencedByOwnedServices,
        IReadOnlySet<string> reclaimableStackIds)
    {
        foreach (var resource in resources.Where(resource => HasLabel(
                     resource.Labels,
                     "com.docker.stack.namespace",
                     stackNamespace)))
        {
            if (HasStackOwnership(resource.Labels, expectedStackId))
            {
                continue;
            }

            if (resourcesReferencedByOwnedServices.Contains(resource.Id)
                && (!HasAnyCitadelOwnershipLabel(resource.Labels)
                    || HasReclaimableStackOwnership(resource.Labels, reclaimableStackIds)))
            {
                continue;
            }

            return $"Swarm Stack namespace '{stackNamespace}' already contains {resourceType} '{resource.Name}' that is not owned by this Citadel Stack.";
        }

        return null;
    }

    private static bool HasAnyCitadelOwnershipLabel(IReadOnlyDictionary<string, string> labels)
        => labels.Keys.Any(static key => key.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase));

    private static bool HasStackOwnership(
        IReadOnlyDictionary<string, string> labels,
        string expectedStackId)
        => HasLabel(labels, CitadelLabels.Managed, "true")
           && HasLabel(labels, CitadelLabels.StackId, expectedStackId)
           && !labels.ContainsKey("com.citadel.service-id")
           && !labels.ContainsKey("com.citadel.deployment-id");

    private static bool HasReclaimableStackOwnership(
        IReadOnlyDictionary<string, string> labels,
        IReadOnlySet<string> reclaimableStackIds)
        => HasLabel(labels, CitadelLabels.Managed, "true")
           && labels.TryGetValue(CitadelLabels.StackId, out var stackId)
           && reclaimableStackIds.Contains(stackId)
           && !labels.ContainsKey("com.citadel.service-id")
           && !labels.ContainsKey("com.citadel.deployment-id");

    private static async Task<SwarmStackConvergenceResult> WaitForSwarmConvergenceAsync(
        ISwarmConnector connector,
        string platformAddress,
        string stackNamespace,
        Guid stackId,
        Guid releaseId,
        IReadOnlySet<string> expectedServices,
        CancellationToken cancellationToken)
    {
        var expectedStackId = stackId.ToString("D");
        var expectedReleaseId = releaseId.ToString("D");

        while (!cancellationToken.IsCancellationRequested)
        {
            var servicesTask = connector.ListServicesAsync(
                new ListSwarmServicesCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
                cancellationToken);
            var tasksTask = connector.ListTasksAsync(
                new ListSwarmTasksCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
                cancellationToken);

            try
            {
                await Task.WhenAll(servicesTask, tasksTask);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                return SwarmStackConvergenceResult.TimedOut;
            }
            catch (Exception ex)
            {
                return new(
                    StackReleaseStatus.Unknown,
                    $"Docker accepted the Stack, but Swarm reconciliation failed: {ex.Message}",
                    []);
            }

            var servicesResult = await servicesTask;
            var tasksResult = await tasksTask;
            if (servicesResult.IsFailure(out var serviceError, out var services))
                return new(StackReleaseStatus.Unknown, $"Docker accepted the Stack, but Service reconciliation failed: {serviceError.Message}", []);
            if (tasksResult.IsFailure(out var taskError, out var tasks))
                return new(StackReleaseStatus.Unknown, $"Docker accepted the Stack, but Task reconciliation failed: {taskError.Message}", []);

            var namespaceServices = services
                .Where(service => HasLabel(service.Labels, "com.docker.stack.namespace", stackNamespace))
                .ToArray();
            var ownedServices = namespaceServices
                .Where(service => HasLabel(service.Labels, CitadelLabels.Managed, "true")
                    && HasLabel(service.Labels, CitadelLabels.StackId, expectedStackId)
                    && HasLabel(service.Labels, CitadelLabels.ReleaseId, expectedReleaseId))
                .ToArray();

            if (namespaceServices.Any(service => !ownedServices.Contains(service)))
            {
                return new(
                    StackReleaseStatus.Failed,
                    $"Swarm Stack namespace '{stackNamespace}' contains a Service whose Citadel ownership does not match release {releaseId:D}.",
                    []);
            }

            var serviceByName = ownedServices.ToDictionary(static service => service.Name, StringComparer.OrdinalIgnoreCase);
            var allExpectedObserved = expectedServices.All(serviceName =>
                serviceByName.ContainsKey($"{stackNamespace}_{serviceName}"));
            if (allExpectedObserved && ownedServices.Length == expectedServices.Count)
            {
                foreach (var service in ownedServices)
                {
                    if (IsPausedRollout(service.UpdateState))
                    {
                        var failure = FindCurrentTaskFailure(service.Id, tasks)
                            ?? service.UpdateMessage
                            ?? $"Docker paused Service '{service.Name}' in rollout state '{service.UpdateState}'.";
                        return new(StackReleaseStatus.Failed, failure, []);
                    }

                    if (IsCompletedRollout(service.UpdateState)
                        && service.RunningTaskCount < service.DesiredTaskCount
                        && FindCurrentTaskFailure(service.Id, tasks) is { } taskFailure)
                    {
                        return new(StackReleaseStatus.Failed, taskFailure, []);
                    }
                }

                if (ownedServices.All(service =>
                        IsCompletedRollout(service.UpdateState)
                        && service.RunningTaskCount >= service.DesiredTaskCount))
                {
                    return SwarmStackConvergenceResult.Healthy(ownedServices);
                }
            }

            try
            {
                await Task.Delay(TimeSpan.FromSeconds(2), cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                return SwarmStackConvergenceResult.TimedOut;
            }
        }

        return SwarmStackConvergenceResult.TimedOut;
    }

    private static bool HasLabel(IReadOnlyDictionary<string, string> labels, string name, string expectedValue)
        => labels.TryGetValue(name, out var value)
            && string.Equals(value, expectedValue, StringComparison.OrdinalIgnoreCase);

    private static bool IsPausedRollout(string state)
        => state.Equals("Paused", StringComparison.OrdinalIgnoreCase)
            || state.Equals("RollbackPaused", StringComparison.OrdinalIgnoreCase)
            || state.Equals("RollbackCompleted", StringComparison.OrdinalIgnoreCase)
            || state.Equals("rollback_paused", StringComparison.OrdinalIgnoreCase)
            || state.Equals("rollback_completed", StringComparison.OrdinalIgnoreCase);

    private static bool IsCompletedRollout(string state)
        => string.IsNullOrWhiteSpace(state)
            || state.Equals("None", StringComparison.OrdinalIgnoreCase)
            || state.Equals("Completed", StringComparison.OrdinalIgnoreCase);

    private static string? FindCurrentTaskFailure(
        string dockerServiceId,
        IReadOnlyList<SwarmTaskResult> tasks)
        => tasks.FirstOrDefault(task =>
            string.Equals(task.ServiceId, dockerServiceId, StringComparison.Ordinal)
            && !task.DesiredState.Equals("shutdown", StringComparison.OrdinalIgnoreCase)
            && !task.DesiredState.Equals("remove", StringComparison.OrdinalIgnoreCase)
            && !string.IsNullOrWhiteSpace(task.Error))?.Error;

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

    private async Task<IReadOnlyList<StackReleaseSwarmResource>> LoadReleaseSwarmResourcesAsync(
        Guid releaseId,
        CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Stacks.GetReleaseSwarmResourcesAsync(releaseId, ct);
    }

    private static async Task<SwarmReleaseResourceCaptureResult> CaptureSwarmReleaseResourcesAsync(
        ISwarmConnector connector,
        string platformAddress,
        Guid platformId,
        string stackNamespace,
        Guid releaseId,
        IReadOnlyList<SwarmServiceResult> ownedServices,
        CancellationToken cancellationToken)
    {
        var referencedSecretIds = ownedServices.SelectMany(static service => service.SecretIds).ToHashSet(StringComparer.Ordinal);
        var referencedConfigIds = ownedServices.SelectMany(static service => service.ConfigIds).ToHashSet(StringComparer.Ordinal);
        if (referencedSecretIds.Count == 0 && referencedConfigIds.Count == 0)
            return new SwarmReleaseResourceCaptureResult([], null);

        IReadOnlyList<SwarmSecretResult> secrets = [];
        if (referencedSecretIds.Count > 0)
        {
            var secretsResult = await connector.ListSecretsAsync(
                new ListSwarmSecretsCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
                cancellationToken);
            if (secretsResult.IsFailure(out var secretError, out var currentSecrets))
                return SwarmReleaseResourceCaptureResult.Failure(secretError.Message);
            secrets = currentSecrets;
        }

        IReadOnlyList<SwarmConfigResult> configs = [];
        if (referencedConfigIds.Count > 0)
        {
            var configsResult = await connector.ListConfigsAsync(
                new ListSwarmConfigsCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
                cancellationToken);
            if (configsResult.IsFailure(out var configError, out var currentConfigs))
                return SwarmReleaseResourceCaptureResult.Failure(configError.Message);
            configs = currentConfigs;
        }

        var allSecretsById = secrets.ToDictionary(static secret => secret.Id, StringComparer.Ordinal);
        var missingSecretId = referencedSecretIds.FirstOrDefault(id => !allSecretsById.ContainsKey(id));
        if (missingSecretId is not null)
            return SwarmReleaseResourceCaptureResult.Failure($"Referenced Swarm Secret '{missingSecretId}' could not be inspected.");

        var allConfigsById = configs.ToDictionary(static config => config.Id, StringComparer.Ordinal);
        var missingConfigId = referencedConfigIds.FirstOrDefault(id => !allConfigsById.ContainsKey(id));
        if (missingConfigId is not null)
            return SwarmReleaseResourceCaptureResult.Failure($"Referenced Swarm Config '{missingConfigId}' could not be inspected.");

        var secretById = allSecretsById.Values
            .Where(secret => HasLabel(secret.Labels, "com.docker.stack.namespace", stackNamespace))
            .ToDictionary(static secret => secret.Id, StringComparer.Ordinal);
        var configById = allConfigsById.Values
            .Where(config => HasLabel(config.Labels, "com.docker.stack.namespace", stackNamespace))
            .ToDictionary(static config => config.Id, StringComparer.Ordinal);
        var mounts = new Dictionary<(StackReleaseSwarmResourceKind Kind, string Id), List<StackReleaseSwarmResourceMount>>();

        foreach (var service in ownedServices)
        {
            var serviceName = RemoveStackNamespace(service.Name, stackNamespace);
            foreach (var secret in service.Definition?.Secrets ?? [])
            {
                if (!secretById.ContainsKey(secret.SecretId))
                    continue;
                AddMount(StackReleaseSwarmResourceKind.Secret, secret.SecretId, serviceName, secret.TargetName);
            }

            foreach (var config in service.Definition?.Configs ?? [])
            {
                if (!configById.ContainsKey(config.ConfigId))
                    continue;
                AddMount(StackReleaseSwarmResourceKind.Config, config.ConfigId, serviceName, config.TargetName);
            }
        }

        var secretWithoutMount = referencedSecretIds.FirstOrDefault(id =>
            secretById.ContainsKey(id)
            && !mounts.ContainsKey((StackReleaseSwarmResourceKind.Secret, id)));
        if (secretWithoutMount is not null)
            return SwarmReleaseResourceCaptureResult.Failure($"Referenced Swarm Secret '{secretWithoutMount}' has no observable Service mount.");

        var configWithoutMount = referencedConfigIds.FirstOrDefault(id =>
            configById.ContainsKey(id)
            && !mounts.ContainsKey((StackReleaseSwarmResourceKind.Config, id)));
        if (configWithoutMount is not null)
            return SwarmReleaseResourceCaptureResult.Failure($"Referenced Swarm Config '{configWithoutMount}' has no observable Service mount.");

        var resources = new List<StackReleaseSwarmResource>(secretById.Count + configById.Count);
        foreach (var secretId in referencedSecretIds)
        {
            if (!secretById.TryGetValue(secretId, out var secret))
                continue;
            resources.Add(new StackReleaseSwarmResource(
                releaseId,
                platformId,
                StackReleaseSwarmResourceKind.Secret,
                secret.Id,
                secret.Name,
                RemoveStackNamespace(secret.Name, stackNamespace),
                mounts.GetValueOrDefault((StackReleaseSwarmResourceKind.Secret, secret.Id)) ?? []));
        }

        foreach (var configId in referencedConfigIds)
        {
            if (!configById.TryGetValue(configId, out var config))
                continue;
            resources.Add(new StackReleaseSwarmResource(
                releaseId,
                platformId,
                StackReleaseSwarmResourceKind.Config,
                config.Id,
                config.Name,
                RemoveStackNamespace(config.Name, stackNamespace),
                mounts.GetValueOrDefault((StackReleaseSwarmResourceKind.Config, config.Id)) ?? []));
        }

        return new SwarmReleaseResourceCaptureResult(resources, null);

        void AddMount(
            StackReleaseSwarmResourceKind kind,
            string resourceId,
            string serviceName,
            string targetName)
        {
            var key = (kind, resourceId);
            if (!mounts.TryGetValue(key, out var values))
            {
                values = [];
                mounts[key] = values;
            }

            var mount = new StackReleaseSwarmResourceMount(serviceName, targetName);
            if (!values.Contains(mount))
                values.Add(mount);
        }
    }

    private async Task<string?> CleanupOldSwarmReleaseResourcesAsync(
        ISwarmConnector connector,
        string platformAddress,
        Guid stackId,
        Guid currentReleaseId,
        IReadOnlyList<StackReleaseSwarmResource> currentResources,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var releases = (await uow.Stacks.GetReleasesByStackIdAsync(stackId, cancellationToken)).ToArray();
        var persistedResources = await uow.Stacks.GetStackSwarmResourcesAsync(stackId, cancellationToken);
        var retainedReleaseIds = releases
            .Where(release => release.Id == currentReleaseId || release.IsRollbackCandidate())
            .OrderByDescending(static release => release.CreatedAt)
            .Take(_retainedSwarmRollbackReleases)
            .Select(static release => release.Id)
            .ToHashSet();
        retainedReleaseIds.Add(currentReleaseId);

        var retainedResourceIds = persistedResources
            .Where(resource => retainedReleaseIds.Contains(resource.StackReleaseId))
            .Select(static resource => (resource.Kind, resource.DockerResourceId))
            .ToHashSet();
        foreach (var resource in currentResources)
            retainedResourceIds.Add((resource.Kind, resource.DockerResourceId));

        var candidates = persistedResources
            .Where(resource => !retainedResourceIds.Contains((resource.Kind, resource.DockerResourceId)))
            .GroupBy(static resource => (resource.Kind, resource.DockerResourceId))
            .Select(static group => group.First())
            .ToArray();
        if (candidates.Length == 0)
            return null;

        var servicesResult = await connector.ListServicesAsync(
            new ListSwarmServicesCommand(platformAddress, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        if (servicesResult.IsFailure(out var serviceError, out var services))
            return $"Old Swarm resource cleanup was skipped: {serviceError.Message}";
        var referencedSecrets = services.SelectMany(static service => service.SecretIds).ToHashSet(StringComparer.Ordinal);
        var referencedConfigs = services.SelectMany(static service => service.ConfigIds).ToHashSet(StringComparer.Ordinal);

        foreach (var candidate in candidates)
        {
            if (candidate.Kind == StackReleaseSwarmResourceKind.Secret)
            {
                if (referencedSecrets.Contains(candidate.DockerResourceId))
                    continue;
                var result = await connector.DeleteSecretAsync(
                    new DeleteSwarmSecretCommand(platformAddress, candidate.DockerResourceId),
                    cancellationToken);
                if (result.IsFailure(out var error))
                {
                    if (error is NotFoundError)
                        continue;
                    return $"Old Swarm Secret cleanup stopped at '{candidate.DockerResourceName}': {error.Message}";
                }
            }
            else
            {
                if (referencedConfigs.Contains(candidate.DockerResourceId))
                    continue;
                var result = await connector.DeleteConfigAsync(
                    new DeleteSwarmConfigCommand(platformAddress, candidate.DockerResourceId),
                    cancellationToken);
                if (result.IsFailure(out var error))
                {
                    if (error is NotFoundError)
                        continue;
                    return $"Old Swarm Config cleanup stopped at '{candidate.DockerResourceName}': {error.Message}";
                }
            }
        }

        return null;
    }

    private async Task<string?> TryCleanupOldSwarmReleaseResourcesAsync(
        ISwarmConnector connector,
        string platformAddress,
        Guid stackId,
        Guid currentReleaseId,
        IReadOnlyList<StackReleaseSwarmResource> currentResources,
        IReadOnlyCollection<string> redactionValues,
        CancellationToken cancellationToken)
    {
        try
        {
            return await CleanupOldSwarmReleaseResourcesAsync(
                connector,
                platformAddress,
                stackId,
                currentReleaseId,
                currentResources,
                cancellationToken);
        }
        catch (OperationCanceledException)
        {
            return "Old Swarm resource cleanup was skipped because its bounded completion window elapsed.";
        }
        catch (Exception ex)
        {
            return $"Old Swarm resource cleanup was skipped: {secretRedactor.Redact(ex.Message, redactionValues)}";
        }
    }

    private static string RemoveStackNamespace(string value, string stackNamespace)
    {
        var prefix = $"{stackNamespace}_";
        return value.StartsWith(prefix, StringComparison.Ordinal)
            ? value[prefix.Length..]
            : value;
    }

    private async Task<bool> TryReserveSwarmNamespaceAsync(
        Guid stackId,
        Guid platformId,
        string stackNamespace,
        CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var reserved = await uow.Stacks.TryReserveSwarmNamespaceAsync(
            stackId,
            platformId,
            stackNamespace,
            ct);
        if (!reserved)
        {
            await uow.RollbackAsync();
            return false;
        }

        await uow.CommitAsync(ct);
        return true;
    }

    private sealed record SwarmLinkedServices(
        IReadOnlySet<string> ServiceIds,
        IReadOnlySet<string> ReclaimableStackIds);

    private async Task<SwarmLinkedServices> LoadSwarmLinkedServicesAsync(
        Guid stackId,
        Guid platformId,
        string stackNamespace,
        CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var services = await uow.Swarm.GetServicesAsync(platformId, ct);
        var linked = services
            .Where(service => service.StackId == stackId
                              && string.Equals(
                                  service.DockerStackNamespace,
                                  stackNamespace,
                                  StringComparison.Ordinal))
            .ToArray();
        var serviceIds = linked
            .Select(static service => service.DockerServiceId)
            .ToHashSet(StringComparer.Ordinal);
        var reclaimableStackIds = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var orphanedOwnerId in linked
                     .Select(static service => service.Labels.TryGetValue(CitadelLabels.StackId, out var value)
                         && Guid.TryParse(value, out var id)
                             ? id
                             : (Guid?)null)
                     .Where(static id => id is not null)
                     .Select(static id => id!.Value)
                     .Where(id => id != stackId)
                     .Distinct())
        {
            if (!await uow.Stacks.ExistsAsync(orphanedOwnerId, ct))
                reclaimableStackIds.Add(orphanedOwnerId.ToString("D"));
        }

        return new SwarmLinkedServices(serviceIds, reclaimableStackIds);
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

internal sealed record SwarmStackConvergenceResult(
    StackReleaseStatus Status,
    string Message,
    IReadOnlyList<SwarmServiceResult> Services)
{
    public static SwarmStackConvergenceResult Healthy(IReadOnlyList<SwarmServiceResult> services) => new(
        StackReleaseStatus.Healthy,
        "Swarm Stack converged successfully.",
        services);

    public static SwarmStackConvergenceResult TimedOut { get; } = new(
        StackReleaseStatus.TimedOut,
        "Docker accepted the Stack, but its Services did not converge within 2 minutes. Reconciliation will continue from the observed Swarm state.",
        []);
}

internal sealed record SwarmReleaseResourceCaptureResult(
    IReadOnlyList<StackReleaseSwarmResource> Resources,
    string? ErrorMessage)
{
    public static SwarmReleaseResourceCaptureResult Failure(string message) => new([], message);
}

internal sealed record TransportSourceBundleResult(
    IReadOnlyList<StackSourceFile> Files,
    string? ErrorMessage)
{
    public static TransportSourceBundleResult Failure(string message) => new([], message);
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
        if (status is StackReleaseStatus.Failed or StackReleaseStatus.TimedOut or StackReleaseStatus.Unknown)
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
    long? operationRowVersion = null,
    IReadOnlyList<StackReleaseSwarmResource>? swarmResources = null) : IDbWorkItem
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
        if (swarmResources is not null)
        {
            await uow.Stacks.ReplaceReleaseSwarmResourcesAsync(
                stack.CurrentStackRelease.Id,
                swarmResources,
                ct);
        }
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
