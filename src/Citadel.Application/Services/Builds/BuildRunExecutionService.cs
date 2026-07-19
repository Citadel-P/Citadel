using Application.Features.Builds.Commands;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Domain.Entities.Registries;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
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
    IBuildRunRetentionService buildRunRetentionService) : IBuildRunExecutionService
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

            if (buildContext.PlatformConnectorType != PlatformConnectorType.Local)
            {
                return await FailAsync(
                    run,
                    BuildRunStatus.Failed,
                    1,
                    "build.runner_not_supported",
                    "Build execution currently supports local Docker platforms. Agent and edge-agent builders require the build connector protocol.",
                    executionToken);
            }

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

            var dockerfilePath = ResolveRepoPath(repositoryRoot, run.DockerfilePath, mustBeDirectory: false);
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
            await unitOfWork.CommitAsync(executionToken);
            await buildRunStreamManager.SendBuildRunInfo(run);
            await SendBuildProjectUpdateAsync(run.BuildProjectId, run, executionToken);

            var command = new BuildProcessCommand(
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
                await unitOfWork.CommitAsync(executionToken);
                await AppendLogAsync(run.Id, "system", "Build run completed successfully.", executionToken);
                await buildRunStreamManager.SendBuildRunInfo(run);
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
            await FlushLogsAsync(bufferedStreamLogs, CancellationToken.None);
            return await CancelAsync(run, CancellationToken.None);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await FlushLogsAsync(bufferedStreamLogs, CancellationToken.None);
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
            await FlushLogsAsync(bufferedStreamLogs, CancellationToken.None);
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
            await FlushLogsAsync(bufferedStreamLogs, CancellationToken.None);
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

        var platform = await unitOfWork.Platforms.GetInfoAsync(run.PlatformSnapshot.Id, cancellationToken);
        if (platform is null)
            return Result.Failure<BuildExecutionContext>(new NotFoundError("Platform not found."));

        var registry = await unitOfWork.Registries.GetAsync(run.RegistrySnapshot.Id, cancellationToken);
        if (registry is null)
            return Result.Failure<BuildExecutionContext>(new NotFoundError("Registry not found."));

        return new BuildExecutionContext(project, repository, platform.ConnectorType, registry);
    }

    private static Result<string> ResolveRepoPath(string repositoryRoot, string relativePath, bool mustBeDirectory)
    {
        if (string.IsNullOrWhiteSpace(relativePath) || relativePath.Contains('\0'))
            return Result.Failure<string>("Build path is invalid.");

        var fullPath = Path.GetFullPath(Path.Combine(repositoryRoot, relativePath));
        var root = EnsureTrailingSeparator(Path.GetFullPath(repositoryRoot));
        var comparison = OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;

        if (!EnsureTrailingSeparator(fullPath).StartsWith(root, comparison) && !fullPath.Equals(root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar), comparison))
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
            if (!IsBuildKitSecretId(buildSecret.Id))
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
        => registry.Configuration switch
        {
            GitHubRegistry github when github.GhcrAuthEnabled == true && !string.IsNullOrWhiteSpace(github.PAT)
                => new BuildProcessRegistryCredential(registryHost, github.NameSpace, github.PAT),
            GitHubRegistry
                => Result.Failure<BuildProcessRegistryCredential?>("GitHub Container Registry pushes require GHCR authentication and a PAT."),
            DockerHubRegistry dockerHub when !string.IsNullOrWhiteSpace(dockerHub.UserName) && !string.IsNullOrWhiteSpace(dockerHub.PAT)
                => new BuildProcessRegistryCredential(registryHost, dockerHub.UserName, dockerHub.PAT),
            DockerHubRegistry
                => Result.Failure<BuildProcessRegistryCredential?>("Docker Hub pushes require a username and PAT."),
            CustomRegistry { AuthEnabled: true } custom when !string.IsNullOrWhiteSpace(custom.UserName) && !string.IsNullOrWhiteSpace(custom.Password)
                => new BuildProcessRegistryCredential(registryHost, custom.UserName, custom.Password),
            CustomRegistry { AuthEnabled: true }
                => Result.Failure<BuildProcessRegistryCredential?>("Custom registry authentication requires a username and password."),
            CustomRegistry
                => Result.Success<BuildProcessRegistryCredential?>(null),
            _
                => Result.Failure<BuildProcessRegistryCredential?>($"Registry type '{registry.Configuration.GetType().Name}' does not support Docker CLI pushes yet.")
        };

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
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
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
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
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

    private async Task AppendLogAsync(Guid runId, string stream, string message, CancellationToken cancellationToken)
    {
        var entry = NewLogEntry(runId, stream, message);
        await unitOfWork.BuildRunLogs.AddAsync(entry, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunLogs(runId, [entry]);
    }

    private async Task FlushLogsAsync(List<BuildRunLogEntry>? entries, CancellationToken cancellationToken)
    {
        if (entries is null || entries.Count == 0)
            return;

        var flushed = entries.ToArray();
        await unitOfWork.BuildRunLogs.AddRangeAsync(flushed, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        entries.Clear();
        await buildRunStreamManager.SendBuildRunLogs(flushed[0].BuildRunId, flushed);
    }

    private static BuildRunLogEntry NewLogEntry(Guid runId, string stream, string message)
        => new(Guid.CreateVersion7(), runId, DateTimeOffset.UtcNow, stream, message);

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

    private static string EnsureTrailingSeparator(string path)
    {
        if (path.EndsWith(Path.DirectorySeparatorChar) || path.EndsWith(Path.AltDirectorySeparatorChar))
            return path;

        return path + Path.DirectorySeparatorChar;
    }

    private static bool IsBuildKitSecretId(string value)
        => !string.IsNullOrWhiteSpace(value)
           && value.All(static ch => char.IsLetterOrDigit(ch) || ch is '.' or '_' or '-');

    private sealed record BuildExecutionContext(
        BuildProject Project,
        Domain.Entities.Git.GitRepository Repository,
        PlatformConnectorType PlatformConnectorType,
        Registry Registry);
}
