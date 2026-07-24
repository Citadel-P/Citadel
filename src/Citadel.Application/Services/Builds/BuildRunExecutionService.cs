using Application.Features.Builds.Commands;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Registries;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Security.Cryptography;

namespace Application.Services.Builds;

public interface IBuildRunExecutionService
{
    ValueTask<Result> ExecuteAsync(Guid runId, CancellationToken cancellationToken);
}

public interface IBuildRunRetentionService
{
    Task PruneAsync(Guid projectId, CancellationToken cancellationToken);
}

public interface IBuildRunCoordinator
{
    CancellationTokenSource Register(Guid runId);
    bool Cancel(Guid runId);
    void Unregister(Guid runId);
}

internal sealed class BuildRunCoordinator : IBuildRunCoordinator
{
    private readonly ConcurrentDictionary<Guid, CoordinatedRun> runs = new();

    public CancellationTokenSource Register(Guid runId)
    {
        var entry = runs.GetOrAdd(runId, static _ => new CoordinatedRun());
        if (Interlocked.Exchange(ref entry.Registered, 1) == 1)
        {
            throw new InvalidOperationException($"Build run {runId} is already registered.");
        }

        return entry.Cancellation;
    }

    public bool Cancel(Guid runId)
    {
        PrunePendingCancellations();

        var entry = runs.GetOrAdd(runId, static _ => new CoordinatedRun());
        entry.CancelledAt = DateTime.UtcNow;
        entry.Cancellation.Cancel();
        return Volatile.Read(ref entry.Registered) == 1;
    }

    public void Unregister(Guid runId)
    {
        if (runs.TryRemove(runId, out var entry))
            entry.Cancellation.Dispose();
    }

    private void PrunePendingCancellations()
    {
        var expiredBefore = DateTime.UtcNow.AddMinutes(-10);
        foreach (var (runId, entry) in runs)
        {
            if (Volatile.Read(ref entry.Registered) == 0 &&
                entry.CancelledAt is { } cancelledAt &&
                cancelledAt < expiredBefore &&
                runs.TryRemove(runId, out var removed))
            {
                removed.Cancellation.Dispose();
            }
        }
    }

    private sealed class CoordinatedRun
    {
        public readonly CancellationTokenSource Cancellation = new();
        public int Registered;
        public DateTime? CancelledAt;
    }
}

internal sealed class BuildRunRetentionService(
    IUnitOfWork unitOfWork,
    IBuildRunStreamManager buildRunStreamManager) : IBuildRunRetentionService
{
    public async Task PruneAsync(Guid projectId, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(projectId, cancellationToken, includeArchived: true);
        if (project is null)
            return;

        var deletedRuns = await unitOfWork.BuildRuns.DeleteTerminalRunsBeyondRetentionAsync(
            projectId,
            project.RetentionRunCount,
            cancellationToken);
        if (deletedRuns.Count == 0)
            return;

        await unitOfWork.CommitAsync(cancellationToken);
        foreach (var run in deletedRuns)
            await buildRunStreamManager.SendBuildRunInfo(run, "delete");
    }
}

internal sealed class BuildRunExecutionService(
    IUnitOfWork unitOfWork,
    IRepoCacheManager repoCacheManager,
    IBuildProcessRunner processRunner,
    IBuildRunCoordinator runCoordinator,
    ISecretValueProtector secretValueProtector,
    IExternalSecretProviderClient externalSecretProviderClient,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IActivityStreamManager activityStreamManager,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    IApplyDeploymentService applyDeploymentService,
    IApplyStackService applyStackService,
    IAlertService alertService,
    IBuildRunRetentionService buildRunRetentionService,
    ILogger<BuildRunExecutionService> logger) : IBuildRunExecutionService
{
    private const int MaxBufferedLogEntries = 25;

    public async ValueTask<Result> ExecuteAsync(Guid runId, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BuildRuns.TryClaimAsync(runId, DateTimeOffset.UtcNow, cancellationToken);
        if (run is null)
            return Result.Success();

        await AppendLogAsync(run.Id, "system", "Build run claimed by worker.", cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
        await SendBuildProjectUpdateAsync(run.BuildProjectId, run, cancellationToken);
        List<BuildRunLogEntry>? bufferedStreamLogs = null;
        var runCancel = runCoordinator.Register(run.Id);

        try
        {
            using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, runCancel.Token);
            var executionToken = linkedCts.Token;

            var context = await LoadContextAsync(run, executionToken);
            if (!context.IsSuccess(out var buildContext, out var loadError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.context_invalid", loadError!.Message, executionToken);

            await AppendLogAsync(run.Id, "system", $"Synchronizing repository \"{buildContext.Repository.Name}\" on branch \"{run.Branch}\".", executionToken);
            var sync = await repoCacheManager.SynchronizeAsync(buildContext.Repository, buildContext.Repository.GitAccount, run.Branch, executionToken);
            if (sync.Success != true || string.IsNullOrWhiteSpace(sync.Hash))
            {
                return await FailAsync(
                    run,
                    BuildRunStatus.Failed,
                    1,
                    "build.git_sync_failed",
                    sync.Error ?? "Git repository synchronization failed.",
                    executionToken);
            }

            var imageReferences = QueueBuildRunHandler.ResolveImageReferences(
                buildContext.Registry.RegistryHost,
                run.ImageRepository,
                run.TagTemplatesSnapshot,
                run.Branch,
                sync.Hash);
            run.ResolveCommit(sync.Hash, imageReferences);
            await unitOfWork.BuildRuns.UpdateAsync(run, executionToken);
            await unitOfWork.CommitAsync(executionToken);

            var repositoryRoot = Path.GetFullPath(buildContext.Repository.GetCachePath());
            var contextPath = ResolveRepoPath(repositoryRoot, run.ContextPath, mustBeDirectory: true);
            if (!contextPath.IsSuccess(out var buildContextPath, out var contextPathError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.context_path_invalid", contextPathError!.Message, executionToken);

            var dockerfilePath = ResolveDockerfilePath(repositoryRoot, buildContextPath, run.DockerfilePath);
            if (!dockerfilePath.IsSuccess(out var dockerfile, out var dockerfileError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.dockerfile_path_invalid", dockerfileError!.Message, executionToken);

            var buildArgs = ResolveBuildArgs(run.BuildArgsSnapshot);
            if (!buildArgs.IsSuccess(out var resolvedBuildArgs, out var buildArgError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.args_invalid", buildArgError!.Message, executionToken);

            var buildSecrets = await ResolveBuildSecretsAsync(buildContext.Project.BuildSecrets, executionToken);
            if (!buildSecrets.IsSuccess(out var resolvedSecrets, out var secretError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.secrets_invalid", secretError!.Message, executionToken);

            var registryHost = NormalizeRegistryHost(buildContext.Registry.RegistryHost);
            var registryCredential = ResolveRegistryCredential(buildContext.Registry, registryHost);
            if (!registryCredential.IsSuccess(out var credential, out var credentialError))
                return await FailAsync(run, BuildRunStatus.Failed, 1, "build.registry_auth_invalid", credentialError!.Message, executionToken);

            await AppendLogAsync(run.Id, "system", $"Building {string.Join(", ", imageReferences)}.", executionToken);
            run.MarkRunning(DateTimeOffset.UtcNow);
            await unitOfWork.BuildRuns.UpdateAsync(run, executionToken);
            var startedActivity = await CreateRunActivityAsync(run, ActivityEventType.BuildRunStarted, new BuildRunStarted(run.Id, run.Trigger), ActivityStatus.Success, executionToken);
            await unitOfWork.ActivityEventRepository.AddAsync(startedActivity, executionToken);
            await unitOfWork.CommitAsync(executionToken);
            await buildRunStreamManager.SendBuildRunInfo(run);
            await activityStreamManager.SendActivityInfo(await startedActivity.AssignActor(unitOfWork, executionToken));
            await SendBuildProjectUpdateAsync(run.BuildProjectId, run, executionToken);

            var command = new BuildProcessCommand(
                PlatformAddress: run.PlatformSnapshot.Address,
                PlatformConnectorType: run.PlatformSnapshot.ConnectorType,
                WorkingDirectory: repositoryRoot,
                ContextPath: buildContextPath,
                DockerfilePath: dockerfile,
                Target: run.Target,
                ImageReferences: imageReferences,
                BuildArgs: resolvedBuildArgs,
                Secrets: resolvedSecrets,
                RegistryCredential: credential,
                Timeout: TimeSpan.FromSeconds(run.TimeoutSeconds));

            int? exitCode = null;
            string? digest = null;
            bufferedStreamLogs = [];
            await foreach (var item in processRunner.RunAsync(command, executionToken))
            {
                if (item.Message is not null)
                {
                    bufferedStreamLogs.Add(NewLogEntry(run.Id, ToLogStream(item.Stream), item.Message));
                    if (bufferedStreamLogs.Count >= MaxBufferedLogEntries)
                        await FlushLogsAsync(bufferedStreamLogs, executionToken);
                }

                if (item.ExitCode.HasValue)
                    exitCode = item.ExitCode.Value;

                if (!string.IsNullOrWhiteSpace(item.Digest))
                    digest = item.Digest;
            }
            await FlushLogsAsync(bufferedStreamLogs, executionToken);

            if (exitCode == 0)
            {
                run.CompleteSucceeded(digest, imageReferences, exitCode, DateTimeOffset.UtcNow);
                await unitOfWork.BuildRuns.UpdateAsync(run, executionToken);
                await unitOfWork.BuildProjects.MarkIdleAsync(run.BuildProjectId, run.Id, executionToken);
                var buildImageConsumers = await ApplyBuildImageConsumersAsync(buildContext.Project, run, executionToken);
                var activity = await CreateRunActivityAsync(
                    run,
                    ActivityEventType.BuildRunSucceeded,
                    new BuildRunSucceeded(run.Id, run.Trigger, run.ExitCode, GetDurationMs(run), run.ImageDigest),
                    ActivityStatus.Success,
                    executionToken);
                await unitOfWork.ActivityEventRepository.AddAsync(activity, executionToken);
                await unitOfWork.CommitAsync(executionToken);

                if (buildImageConsumers.LogEntries.Length > 0)
                    await SendBuildRunLogsSafeAsync(run.Id, buildImageConsumers.LogEntries);
                await AppendLogAsync(run.Id, "system", "Build run completed successfully.", executionToken);
                await buildRunStreamManager.SendBuildRunInfo(run);
                await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, executionToken));
                foreach (var notification in buildImageConsumers.DeploymentNotifications)
                {
                    await deploymentStreamManager.SendDeploymentInfo(notification.Deployment);
                    await activityStreamManager.SendActivityInfo(await notification.Activity.AssignActor(unitOfWork, executionToken));
                }

                foreach (var notification in buildImageConsumers.StackNotifications)
                {
                    await stackStreamManager.SendStackInfo(notification.Stack);
                    await activityStreamManager.SendActivityInfo(await notification.Activity.AssignActor(unitOfWork, executionToken));
                }

                await TryRedeployBuildImageConsumersAsync(run.Id, buildImageConsumers, run.TriggeredByActorId, executionToken);

                await buildRunRetentionService.PruneAsync(run.BuildProjectId, executionToken);
                await SendBuildProjectUpdateAsync(run.BuildProjectId, run, executionToken);
                return Result.Success();
            }

            var status = exitCode == -2 ? BuildRunStatus.TimedOut : BuildRunStatus.Failed;
            return await FailAsync(
                run,
                status,
                exitCode,
                status == BuildRunStatus.TimedOut ? "build.timed_out" : "build.docker_failed",
                status == BuildRunStatus.TimedOut ? "Docker build timed out." : "Docker build or push failed.",
                executionToken);
        }
        catch (OperationCanceledException) when (runCancel.IsCancellationRequested)
        {
            await ResetTransactionAsync();
            await TryFlushLogsForRecoveryAsync(bufferedStreamLogs);
            return await CancelAsync(run, CancellationToken.None);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await ResetTransactionAsync();
            await TryFlushLogsForRecoveryAsync(bufferedStreamLogs);
            return await FailAsync(
                run,
                BuildRunStatus.Interrupted,
                null,
                "build.interrupted",
                "Build run was interrupted.",
                CancellationToken.None);
        }
        catch (OperationCanceledException)
        {
            await ResetTransactionAsync();
            await TryFlushLogsForRecoveryAsync(bufferedStreamLogs);
            return await FailAsync(
                run,
                BuildRunStatus.TimedOut,
                -2,
                "build.timed_out",
                "Docker build timed out.",
                CancellationToken.None);
        }
        catch (Exception ex)
        {
            await ResetTransactionAsync();
            await TryFlushLogsForRecoveryAsync(bufferedStreamLogs);
            return await FailAsync(
                run,
                BuildRunStatus.Failed,
                1,
                "build.unhandled_error",
                ex.Message,
                CancellationToken.None);
        }
        finally
        {
            runCoordinator.Unregister(run.Id);
        }
    }

    private async Task<Result<BuildExecutionContext>> LoadContextAsync(BuildRun run, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(run.BuildProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildExecutionContext>(new NotFoundError("Build project not found."));

        var repository = await unitOfWork.GitRepositories.GetWithAccountAsync(run.GitRepositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<BuildExecutionContext>(new NotFoundError("Git repository not found."));

        var target = await ResolveConnectorTypeAsync(run, cancellationToken);
        if (!target.IsSuccess(out var connectorType, out var targetError))
            return Result.Failure<BuildExecutionContext>(targetError!);

        var registry = await unitOfWork.Registries.GetAsync(run.RegistrySnapshot.Id, cancellationToken);
        if (registry is null)
            return Result.Failure<BuildExecutionContext>(new NotFoundError("Registry not found."));

        return new BuildExecutionContext(project, repository, connectorType, registry);
    }

    private async Task<Result<PlatformConnectorType>> ResolveConnectorTypeAsync(
        BuildRun run,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetInfoAsync(run.PlatformSnapshot.Id, cancellationToken);
        if (platform is not null)
            return platform.ConnectorType;

        var pool = await unitOfWork.BuildAgentPools.GetAsync(run.PlatformSnapshot.Id, cancellationToken, includeArchived: true);
        if (pool is null)
            return Result.Failure<PlatformConnectorType>(new NotFoundError("Build target not found."));

        if (pool.ProviderSpec is not SelfManagedVmBuildAgentPoolProviderSpec)
            return Result.Failure<PlatformConnectorType>(
                new ConflictError("Build pool provider is not runnable by the self-managed Citadel Agent executor."));

        return run.PlatformSnapshot.ConnectorType;
    }

    private static Result<string> ResolveRepoPath(string repositoryRoot, string relativePath, bool mustBeDirectory)
    {
        if (string.IsNullOrWhiteSpace(relativePath) || relativePath.Contains('\0'))
            return Result.Failure<string>("Build path is invalid.");

        var fullPath = Path.GetFullPath(Path.Combine(repositoryRoot, relativePath));
        var root = EnsureTrailingSeparator(Path.GetFullPath(repositoryRoot));
        var comparison = OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;

        if (!IsInsideOrEqual(fullPath, root, comparison))
            return Result.Failure<string>("Build path must stay inside the repository cache.");

        if (mustBeDirectory)
        {
            if (!Directory.Exists(fullPath))
                return Result.Failure<string>($"Build context path '{relativePath}' does not exist.");
        }
        else if (!File.Exists(fullPath))
        {
            return Result.Failure<string>($"Dockerfile path '{relativePath}' does not exist.");
        }

        return fullPath;
    }

    private static Result<string> ResolveDockerfilePath(string repositoryRoot, string contextDirectory, string dockerfilePath)
    {
        if (string.IsNullOrWhiteSpace(dockerfilePath) || dockerfilePath.Contains('\0'))
            return Result.Failure<string>("Build path is invalid.");

        var root = EnsureTrailingSeparator(Path.GetFullPath(repositoryRoot));
        var comparison = OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;
        var repoRelativePath = Path.GetFullPath(Path.Combine(repositoryRoot, dockerfilePath));
        if (!IsInsideOrEqual(repoRelativePath, root, comparison))
            return Result.Failure<string>("Build path must stay inside the repository cache.");

        if (File.Exists(repoRelativePath))
            return repoRelativePath;

        var contextRelativePath = Path.GetFullPath(Path.Combine(contextDirectory, dockerfilePath));
        if (!IsInsideOrEqual(contextRelativePath, root, comparison))
            return Result.Failure<string>("Build path must stay inside the repository cache.");

        return File.Exists(contextRelativePath)
            ? contextRelativePath
            : Result.Failure<string>($"Dockerfile path '{dockerfilePath}' does not exist.");
    }

    private static bool IsInsideOrEqual(string path, string root, StringComparison comparison)
        => EnsureTrailingSeparator(path).StartsWith(root, comparison) ||
           path.Equals(root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar), comparison);

    private static Result<IReadOnlyList<BuildProcessBuildArg>> ResolveBuildArgs(IReadOnlyList<BuildArgSpec> buildArgs)
    {
        var resolved = new List<BuildProcessBuildArg>();
        foreach (var buildArg in buildArgs)
        {
            if (string.IsNullOrWhiteSpace(buildArg.Name) || buildArg.Name.Contains('='))
                return Result.Failure<IReadOnlyList<BuildProcessBuildArg>>($"Build arg '{buildArg.Name}' has an invalid name.");

            if (buildArg.ResourceBindingId.HasValue)
                return Result.Failure<IReadOnlyList<BuildProcessBuildArg>>($"Build arg '{buildArg.Name}' references a resource binding. Resource-bound build args are not supported yet.");

            resolved.Add(new BuildProcessBuildArg(buildArg.Name, buildArg.Value ?? string.Empty));
        }

        return resolved;
    }

    private async Task<Result<IReadOnlyList<BuildProcessSecret>>> ResolveBuildSecretsAsync(
        IReadOnlyList<BuildSecretSpec> buildSecrets,
        CancellationToken cancellationToken)
    {
        if (buildSecrets.Count == 0)
            return Array.Empty<BuildProcessSecret>();

        var materials = await unitOfWork.SecretDefinitions.GetResolutionMaterialsAsync(
            [.. buildSecrets.Select(static x => x.SecretId)],
            cancellationToken);
        var materialById = materials.ToDictionary(static x => x.Definition.Id);
        var providerTokens = new Dictionary<Guid, string>();
        var resolved = new List<BuildProcessSecret>();

        foreach (var buildSecret in buildSecrets)
        {
            if (!BuildProject.IsBuildKitSecretId(buildSecret.Id))
                return Result.Failure<IReadOnlyList<BuildProcessSecret>>($"Build secret id '{buildSecret.Id}' is invalid. Use letters, numbers, '.', '_' or '-'.");

            if (!materialById.TryGetValue(buildSecret.SecretId, out var material))
                return Result.Failure<IReadOnlyList<BuildProcessSecret>>($"Build secret '{buildSecret.Id}' is not available.");

            var plaintext = await ResolveSecretAsync(material, buildSecret.Id, providerTokens, cancellationToken);
            if (!plaintext.IsSuccess(out var value, out var error))
                return Result.Failure<IReadOnlyList<BuildProcessSecret>>(error!);

            resolved.Add(new BuildProcessSecret(buildSecret.Id, value));
        }

        return resolved;
    }

    private async Task<Result<string>> ResolveSecretAsync(
        SecretResolutionMaterial material,
        string buildSecretId,
        IDictionary<Guid, string> providerTokens,
        CancellationToken cancellationToken)
    {
        var secret = material.Definition;
        if (secret.ProviderType == SecretProviderType.InternalEncrypted)
        {
            if (material.InternalValue is null)
                return Result.Failure<string>($"Build secret '{buildSecretId}' is not available.");

            return Unprotect(material.InternalValue.EncryptedValue, $"Build secret '{buildSecretId}' could not be decrypted.");
        }

        if (secret.ProviderId is null)
            return Result.Failure<string>($"Build secret '{buildSecretId}' does not reference a provider.");

        if (material.Provider is null)
            return Result.Failure<string>($"Build secret provider for '{buildSecretId}' is not available.");

        if (!providerTokens.TryGetValue(secret.ProviderId.Value, out var token))
        {
            var tokenResult = Unprotect(
                material.Provider.Configuration.ProtectedToken,
                $"Build secret provider token for '{buildSecretId}' could not be decrypted.");
            if (!tokenResult.IsSuccess(out token, out var tokenError))
                return Result.Failure<string>(tokenError!);

            providerTokens[secret.ProviderId.Value] = token;
        }

        var external = await externalSecretProviderClient.ResolveAsync(secret, material.Provider, token, cancellationToken);
        return external.IsSuccess
            ? external.Value ?? string.Empty
            : Result.Failure<string>(external.ErrorMessage ?? $"Build secret '{buildSecretId}' could not be resolved.");
    }

    private Result<string> Unprotect(string value, string message)
    {
        try
        {
            return secretValueProtector.Unprotect(value);
        }
        catch (Exception ex) when (ex is FormatException or CryptographicException)
        {
            return Result.Failure<string>(message);
        }
    }

    private static Result<BuildProcessRegistryCredential?> ResolveRegistryCredential(Registry registry, string registryHost)
    {
        var credentialResult = registry.Configuration switch
        {
            GitHubRegistry github when github.GhcrAuthEnabled == true && !string.IsNullOrWhiteSpace(github.PAT)
                => Result.Success<string?>(github.GetRegistryAuth(registryHost)),
            GitHubRegistry
                => Result.Failure<string?>("GitHub Container Registry pushes require GHCR authentication and a PAT."),
            DockerHubRegistry dockerHub when !string.IsNullOrWhiteSpace(dockerHub.UserName) && !string.IsNullOrWhiteSpace(dockerHub.PAT)
                => Result.Success<string?>(dockerHub.GetRegistryAuth(registryHost)),
            DockerHubRegistry
                => Result.Failure<string?>("Docker Hub pushes require a username and PAT."),
            CustomRegistry { AuthEnabled: true } custom when !string.IsNullOrWhiteSpace(custom.UserName) && !string.IsNullOrWhiteSpace(custom.Password)
                => Result.Success<string?>(custom.GetRegistryAuth(registryHost)),
            CustomRegistry { AuthEnabled: true }
                => Result.Failure<string?>("Custom registry authentication requires a username and password."),
            CustomRegistry
                => Result.Success<string?>(null),
            _
                => Result.Failure<string?>($"Registry type '{registry.Configuration.GetType().Name}' does not support Docker image pushes yet.")
        };

        if (!credentialResult.IsSuccess(out var registryAuth, out var error))
            return Result.Failure<BuildProcessRegistryCredential?>(error!);

        return string.IsNullOrWhiteSpace(registryAuth)
            ? Result.Success<BuildProcessRegistryCredential?>(null)
            : new BuildProcessRegistryCredential(registryHost, registryAuth);
    }

    private async Task<Result> FailAsync(
        BuildRun run,
        BuildRunStatus status,
        int? exitCode,
        string errorCode,
        string errorMessage,
        CancellationToken cancellationToken)
    {
        await AppendLogAsync(run.Id, "stderr", errorMessage, cancellationToken);
        run.Fail(status, exitCode, errorCode, errorMessage, DateTimeOffset.UtcNow);
        await unitOfWork.BuildRuns.UpdateAsync(run, cancellationToken);
        await unitOfWork.BuildProjects.MarkIdleAsync(run.BuildProjectId, run.Id, cancellationToken);
        var activity = status == BuildRunStatus.TimedOut
            ? await CreateRunActivityAsync(
                run,
                ActivityEventType.BuildRunTimedOut,
                new BuildRunTimedOut(run.Id, run.Trigger, GetDurationMs(run), errorMessage),
                ActivityStatus.Failure,
                cancellationToken)
            : await CreateRunActivityAsync(
                run,
                ActivityEventType.BuildRunFailed,
                new BuildRunFailed(run.Id, run.Trigger, status, run.ExitCode, GetDurationMs(run), errorMessage),
                ActivityStatus.Failure,
                cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await alertService.ProcessAsync(
            AlertType.BuildRunFailed,
            new AlertEvaluationContext(
                DateTime.UtcNow,
                Platforms: [],
                Deployments: [],
                Stacks: [],
                BuildRunFailures:
                [
                    new BuildRunFailureAlertSnapshot(
                        run.BuildProjectId,
                        run.ProjectNameSnapshot,
                        run.Id,
                        run.Trigger,
                        status,
                        run.ExitCode,
                        GetDurationMs(run),
                        errorMessage)
                ]),
            cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        await buildRunRetentionService.PruneAsync(run.BuildProjectId, cancellationToken);
        await SendBuildProjectUpdateAsync(run.BuildProjectId, run, cancellationToken);
        return Result.Failure(new BadGatewayError(errorMessage));
    }

    private async Task<Result> CancelAsync(BuildRun run, CancellationToken cancellationToken)
    {
        await AppendLogAsync(run.Id, "stderr", "Build run cancelled.", cancellationToken);
        run.Cancel(DateTimeOffset.UtcNow);
        await unitOfWork.BuildRuns.UpdateAsync(run, cancellationToken);
        await unitOfWork.BuildProjects.MarkIdleAsync(run.BuildProjectId, run.Id, cancellationToken);
        var activity = await CreateRunActivityAsync(
            run,
            ActivityEventType.BuildRunCancelled,
            new BuildRunCancelled(run.Id, run.Trigger),
            ActivityStatus.Warning,
            cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        await buildRunRetentionService.PruneAsync(run.BuildProjectId, cancellationToken);
        await SendBuildProjectUpdateAsync(run.BuildProjectId, run, cancellationToken);
        return Result.Success();
    }

    private async Task SendBuildProjectUpdateAsync(Guid projectId, BuildRun latestRun, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(projectId, cancellationToken);
        if (project is not null)
            await buildProjectStreamManager.SendBuildProjectInfo(project, latestRun: latestRun);
    }

    private async Task<BuildImageConsumerUpdateResult> ApplyBuildImageConsumersAsync(
        BuildProject project,
        BuildRun run,
        CancellationToken cancellationToken)
    {
        var imageReference = run.ImageReferences.FirstOrDefault();
        if (string.IsNullOrWhiteSpace(imageReference))
            return new BuildImageConsumerUpdateResult([], [], [], [], []);

        var deploymentNotifications = new List<DeploymentConsumerNotification>();
        var stackNotifications = new List<StackConsumerNotification>();
        var deploymentsToRedeploy = new List<Guid>();
        var stacksToRedeploy = new List<StackRedeployRequest>();
        var logEntries = new List<BuildRunLogEntry>();

        var deployments = await unitOfWork.Deployments.GetBuildImageConsumersAsync(project.Id, cancellationToken);
        foreach (var deployment in deployments)
        {
            if (deployment.Spec?.Image is not BuildImage buildImage || buildImage.BuildProjectId != project.Id)
                continue;

            var oldSnapshot = deployment.ToSnapshot();
            deployment.PartialUpdate(
                spec: deployment.Spec with
                {
                    Image = buildImage with
                    {
                        ResolvedImageReference = imageReference,
                        ResolvedDigest = run.ImageDigest
                    }
                });

            var activity = new ActivityEvent(
                platformId: deployment.PlatformId,
                resourceId: deployment.Id,
                actorId: run.TriggeredByActorId,
                resourceName: deployment.Name,
                eventType: ActivityEventType.DeploymentUpdated,
                status: ActivityStatus.Success,
                info: new DeploymentUpdated(oldSnapshot, deployment.ToSnapshot()));

            await unitOfWork.Deployments.UpdateAsync(deployment, cancellationToken);
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            var logEntry = NewLogEntry(run.Id, "system", $"Updated build image source for deployment \"{deployment.Name}\" to {imageReference}.");
            await unitOfWork.BuildRunLogs.AddAsync(logEntry, cancellationToken);
            logEntries.Add(logEntry);
            deploymentNotifications.Add(new DeploymentConsumerNotification(deployment, activity));

            if (buildImage.RedeployOnBuild)
                deploymentsToRedeploy.Add(deployment.Id);
        }

        var stacks = await unitOfWork.Stacks.GetBuildImageConsumerStacksAsync(project.Id, cancellationToken);
        foreach (var stack in stacks)
        {
            var release = stack.CurrentStackRelease;
            var bindings = release?.Spec?.BuildImageBindings;
            if (release?.Spec is null || bindings is not { Count: > 0 })
                continue;

            var changed = false;
            var redeployServices = new List<string>();
            var updatedServices = new List<string>();
            var nextBindings = bindings.Select(binding =>
            {
                if (binding.BuildProjectId != project.Id)
                    return binding;

                changed = true;
                updatedServices.Add(binding.ServiceName);
                if (binding.RedeployOnBuild)
                    redeployServices.Add(binding.ServiceName);

                return binding with
                {
                    ResolvedImageReference = imageReference,
                    ResolvedDigest = run.ImageDigest
                };
            }).ToArray();

            if (!changed)
                continue;

            var oldSnapshot = stack.ToSnapshot();
            release.UpdateSpec(SetBuildImageBindings(release.Spec, nextBindings));
            var activity = new ActivityEvent(
                platformId: release.PlatformId,
                resourceId: stack.Id,
                actorId: run.TriggeredByActorId,
                resourceName: stack.Name,
                eventType: ActivityEventType.StackUpdated,
                status: ActivityStatus.Success,
                info: new StackUpdated(oldSnapshot, stack.ToSnapshot()));

            await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            var logEntry = NewLogEntry(
                run.Id,
                "system",
                $"Updated build image binding for stack \"{stack.Name}\" service(s) {string.Join(", ", updatedServices)} to {imageReference}.");
            await unitOfWork.BuildRunLogs.AddAsync(logEntry, cancellationToken);
            logEntries.Add(logEntry);
            stackNotifications.Add(new StackConsumerNotification(stack, activity));

            if (redeployServices.Count > 0)
                stacksToRedeploy.Add(new StackRedeployRequest(stack.Id, [.. redeployServices.Distinct(StringComparer.OrdinalIgnoreCase)]));
        }

        return new BuildImageConsumerUpdateResult(
            deploymentNotifications,
            stackNotifications,
            deploymentsToRedeploy,
            stacksToRedeploy,
            [.. logEntries]);
    }

    private async Task TryRedeployBuildImageConsumersAsync(
        Guid runId,
        BuildImageConsumerUpdateResult consumers,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        foreach (var deploymentId in consumers.DeploymentsToRedeploy)
        {
            try
            {
                await foreach (var _ in applyDeploymentService.ApplyAsync(deploymentId, actorId, recreate: false, ct: cancellationToken))
                {
                }
            }
            catch (OperationCanceledException ex)
            {
                logger.LogWarning(ex, "Deployment redeploy for {DeploymentId} was cancelled after build run {BuildRunId} completed.", deploymentId, runId);
                await AppendPostBuildRedeployLogAsync(runId, $"Deployment redeploy was cancelled for {deploymentId}.");
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                logger.LogWarning(ex, "Failed to redeploy deployment {DeploymentId} after build run {BuildRunId}.", deploymentId, runId);
                await AppendPostBuildRedeployLogAsync(runId, $"Deployment redeploy failed for {deploymentId}: {ex.Message}");
            }
        }

        foreach (var request in consumers.StacksToRedeploy)
        {
            try
            {
                await foreach (var _ in applyStackService.ApplyAsync(
                    request.StackId,
                    actorId,
                    request.ServiceNames,
                    pullImages: true,
                    recreate: false,
                    waitForCompletion: false,
                    operation: StackApplyOperation.Apply,
                    previousStackSnapshot: null,
                    ct: cancellationToken))
                {
                }
            }
            catch (OperationCanceledException ex)
            {
                logger.LogWarning(ex, "Stack redeploy for {StackId} was cancelled after build run {BuildRunId} completed.", request.StackId, runId);
                await AppendPostBuildRedeployLogAsync(runId, $"Stack redeploy was cancelled for {request.StackId}.");
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                logger.LogWarning(ex, "Failed to redeploy stack {StackId} after build run {BuildRunId}.", request.StackId, runId);
                await AppendPostBuildRedeployLogAsync(runId, $"Stack redeploy failed for {request.StackId}: {ex.Message}");
            }
        }
    }

    private async Task AppendPostBuildRedeployLogAsync(Guid runId, string message)
    {
        var entry = NewLogEntry(runId, "stderr", message);
        try
        {
            await unitOfWork.BuildRunLogs.AddAsync(entry, CancellationToken.None);
            await unitOfWork.CommitAsync(CancellationToken.None);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to persist post-build redeploy log for build run {BuildRunId}.", runId);
            await ResetTransactionAsync();
        }

        await SendBuildRunLogsSafeAsync(runId, [entry]);
    }

    private static StackSpec SetBuildImageBindings(StackSpec spec, IReadOnlyList<StackBuildImageBinding> bindings)
        => spec switch
        {
            ManualStack manual => manual with { BuildImageBindings = bindings },
            GitStack git => git with { BuildImageBindings = bindings },
            _ => spec
        };

    private async Task AppendLogAsync(Guid runId, string stream, string message, CancellationToken cancellationToken)
    {
        var entry = NewLogEntry(runId, stream, message);
        await unitOfWork.BuildRunLogs.AddAsync(entry, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await SendBuildRunLogsSafeAsync(runId, new[] { entry });
    }

    private async Task FlushLogsAsync(List<BuildRunLogEntry>? entries, CancellationToken cancellationToken)
    {
        if (entries is null || entries.Count == 0)
            return;

        var flushed = entries.ToArray();
        await unitOfWork.BuildRunLogs.AddRangeAsync(flushed, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        entries.Clear();
        await SendBuildRunLogsSafeAsync(flushed[0].BuildRunId, flushed);
    }

    private async Task TryFlushLogsForRecoveryAsync(List<BuildRunLogEntry>? entries)
    {
        try
        {
            await FlushLogsAsync(entries, CancellationToken.None);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to flush buffered build run logs during recovery.");
            await ResetTransactionAsync();
        }
    }

    private async Task ResetTransactionAsync()
    {
        try
        {
            await unitOfWork.RollbackAsync();
        }
        catch (Exception rollbackError)
        {
            logger.LogWarning(rollbackError, "Failed to rollback build run transaction before writing recovery state.");
        }
    }

    private async Task SendBuildRunLogsSafeAsync(Guid runId, BuildRunLogEntry[] entries)
    {
        try
        {
            await buildRunStreamManager.SendBuildRunLogs(runId, entries);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to stream build run logs for {BuildRunId}. Persisted logs are still available.", runId);
        }
    }

    private static BuildRunLogEntry NewLogEntry(Guid runId, string stream, string message)
        => new(Guid.CreateVersion7(), runId, DateTimeOffset.UtcNow, stream, RemovePostgresNullBytes(message));

    private static string RemovePostgresNullBytes(string value)
        => value.Contains('\0', StringComparison.Ordinal)
            ? value.Replace("\0", string.Empty, StringComparison.Ordinal)
            : value;

    private static string NormalizeRegistryHost(string registryHost)
        => registryHost
            .Replace("https://", "", StringComparison.OrdinalIgnoreCase)
            .Replace("http://", "", StringComparison.OrdinalIgnoreCase)
            .TrimEnd('/')
            .ToLowerInvariant();

    private static string ToLogStream(BuildProcessStream stream)
        => stream switch
        {
            BuildProcessStream.StdErr => "stderr",
            BuildProcessStream.Exit => "system",
            _ => "stdout"
        };

    private async Task<ActivityEvent> CreateRunActivityAsync(
        BuildRun run,
        ActivityEventType eventType,
        ActivityEventInfo info,
        ActivityStatus status,
        CancellationToken cancellationToken)
    {
        var platformId = await ResolveActivityPlatformIdAsync(run, cancellationToken);
        return new ActivityEvent(
            platformId: platformId,
            resourceId: run.BuildProjectId,
            actorId: run.TriggeredByActorId,
            resourceName: run.ProjectNameSnapshot,
            eventType: eventType,
            status: status,
            info: info);
    }

    private async Task<Guid?> ResolveActivityPlatformIdAsync(BuildRun run, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetInfoAsync(run.PlatformSnapshot.Id, cancellationToken);
        return platform?.Id;
    }

    private static long? GetDurationMs(BuildRun run)
    {
        if (run.StartedAt is null || run.CompletedAt is null)
            return null;

        return Math.Max(0, (long)(run.CompletedAt.Value - run.StartedAt.Value).TotalMilliseconds);
    }

    private static string EnsureTrailingSeparator(string path)
    {
        if (path.EndsWith(Path.DirectorySeparatorChar) || path.EndsWith(Path.AltDirectorySeparatorChar))
            return path;

        return path + Path.DirectorySeparatorChar;
    }

    private sealed record DeploymentConsumerNotification(Deployment Deployment, ActivityEvent Activity);

    private sealed record BuildImageConsumerUpdateResult(
        IReadOnlyList<DeploymentConsumerNotification> DeploymentNotifications,
        IReadOnlyList<StackConsumerNotification> StackNotifications,
        IReadOnlyList<Guid> DeploymentsToRedeploy,
        IReadOnlyList<StackRedeployRequest> StacksToRedeploy,
        BuildRunLogEntry[] LogEntries);

    private sealed record StackConsumerNotification(Stack Stack, ActivityEvent Activity);

    private sealed record StackRedeployRequest(Guid StackId, IReadOnlyList<string> ServiceNames);

    private sealed record BuildExecutionContext(
        BuildProject Project,
        Domain.Entities.Git.GitRepository Repository,
        PlatformConnectorType PlatformConnectorType,
        Registry Registry);
}
