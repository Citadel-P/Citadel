using Application.Configs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Backups;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;

namespace Application.Services.Backups;

public interface IBackupRunExecutionService
{
    IAsyncEnumerable<BackupRunStreamItem> ExecuteQueuedAsync(Guid runId, CancellationToken cancellationToken);
    IAsyncEnumerable<BackupRunStreamItem> ExecuteAsync(Guid runId, CancellationToken cancellationToken);
}

public interface IBackupRunCoordinator
{
    CancellationTokenSource Register(Guid runId);
    bool Cancel(Guid runId);
    void Unregister(Guid runId);
}

public sealed record BackupRunStreamItem(
    Guid RunId,
    BackupRunStatus? Status,
    string? Message,
    string? Stream = null,
    int? ExitCode = null);

internal sealed class BackupRunCoordinator : IBackupRunCoordinator
{
    private readonly ConcurrentDictionary<Guid, CancellationTokenSource> runs = new();

    public CancellationTokenSource Register(Guid runId)
    {
        var cts = new CancellationTokenSource();
        if (!runs.TryAdd(runId, cts))
        {
            cts.Dispose();
            throw new InvalidOperationException($"Backup run {runId} is already registered.");
        }

        return cts;
    }

    public bool Cancel(Guid runId)
    {
        if (!runs.TryGetValue(runId, out var cts))
            return false;

        cts.Cancel();
        return true;
    }

    public void Unregister(Guid runId)
    {
        if (runs.TryRemove(runId, out var cts))
            cts.Dispose();
    }
}

internal sealed class BackupRunExecutionService(
    IServiceScopeFactory scopeFactory,
    IResticEnvironmentBuilder environmentBuilder,
    IResticProcessRunner processRunner,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    IBackupRunCoordinator runCoordinator,
    IOptions<BackupOptions> backupOptions,
    ILogger<BackupRunExecutionService> logger) : IBackupRunExecutionService
{
    private const int LogBatchSize = 25;
    private readonly BackupOptions options = backupOptions.Value;

    public async IAsyncEnumerable<BackupRunStreamItem> ExecuteQueuedAsync(
        Guid runId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var plan = await TryClaimExecutionPlanAsync(runId, cancellationToken);
        if (plan is null)
        {
            yield return Error(runId, BackupRunStatus.Rejected, "Backup run is no longer queued.");
            yield break;
        }

        await foreach (var item in ExecutePlanAsync(plan, cancellationToken))
            yield return item;
    }

    public async IAsyncEnumerable<BackupRunStreamItem> ExecuteAsync(
        Guid runId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var plan = await LoadExecutionPlanAsync(runId, cancellationToken);
        if (plan is null)
            yield break;

        await foreach (var item in ExecutePlanAsync(plan, cancellationToken))
            yield return item;
    }

    private async IAsyncEnumerable<BackupRunStreamItem> ExecutePlanAsync(
        BackupRunExecutionPlan plan,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var channel = Channel.CreateUnbounded<BackupRunStreamItem>(
            new UnboundedChannelOptions
            {
                SingleReader = true,
                SingleWriter = true
            });

        var producer = Task.Run(
            () => RunExecutionProducerAsync(plan, channel.Writer, cancellationToken),
            CancellationToken.None);

        await foreach (var item in channel.Reader.ReadAllAsync(cancellationToken))
            yield return item;

        await producer;
    }

    private async Task RunExecutionProducerAsync(
        BackupRunExecutionPlan plan,
        ChannelWriter<BackupRunStreamItem> writer,
        CancellationToken cancellationToken)
    {
        try
        {
            await ExecuteCoreAsync(plan, writer, cancellationToken);
        }
        catch (Exception ex)
        {
            var runId = plan.Run.Id;
            logger.LogError(ex, "Backup run {RunId} failed before the execution context was loaded.", runId);
            writer.TryComplete(ex);
        }
    }

    private async Task ExecuteCoreAsync(
        BackupRunExecutionPlan context,
        ChannelWriter<BackupRunStreamItem> writer,
        CancellationToken cancellationToken)
    {
        var run = context.Run;
        var policy = context.Policy;
        var repository = context.Repository;
        var operationToken = runCoordinator.Register(run.Id);

        try
        {
            using var timeoutCancel = new CancellationTokenSource(TimeSpan.FromSeconds(Math.Max(60, policy.TimeoutSeconds)));
            using var linkedCancel = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCancel.Token, operationToken.Token);

            await WriteAsync(writer, Info(run.Id, BackupRunStatus.Preparing, $"Backup \"{policy.Name}\" is preparing."), cancellationToken);

            if (!options.Enabled)
            {
                await FailRunAsync(run, policy, BackupRunStatus.Rejected, null, "backup.disabled", "Backups are disabled.", cancellationToken);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.Rejected, "Backups are disabled."), cancellationToken);
                return;
            }

            var repositoryLease = await AcquireRepositoryLeaseAsync(run, linkedCancel.Token);
            if (!repositoryLease)
            {
                const string message = "Backup repository already has an active operation.";
                await FailRunAsync(run, policy, BackupRunStatus.Rejected, null, "backup.repository_busy", message, cancellationToken);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.Rejected, message), cancellationToken);
                return;
            }

            var sourceLease = false;
            try
            {
                sourceLease = await AcquireSourceLeaseAsync(run, linkedCancel.Token);
                if (!sourceLease)
                {
                    const string message = "Backup source already has an active operation.";
                    await FailRunAsync(run, policy, BackupRunStatus.Rejected, null, "backup.source_busy", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRunStatus.Rejected, message), cancellationToken);
                    return;
                }

                var sourcePlan = await ResolveSourceAsync(run.SourceSnapshot, linkedCancel.Token);
                if (!sourcePlan.IsSuccess(out var source, out var sourceError))
                {
                    var message = sourceError?.Message ?? "Backup source could not be resolved.";
                    await FailRunAsync(run, policy, BackupRunStatus.Rejected, null, "backup.source_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRunStatus.Rejected, message), cancellationToken);
                    return;
                }

                var environmentResult = await BuildEnvironmentAsync(repository, source.Context, linkedCancel.Token);
                if (!environmentResult.IsSuccess(out var environment, out var environmentError))
                {
                    var message = environmentError?.Message ?? "Backup repository could not be prepared.";
                    await FailRunAsync(run, policy, BackupRunStatus.Failed, null, "backup.repository_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRunStatus.Failed, message), cancellationToken);
                    return;
                }

                await using (environment)
                {
                    run.MarkRunning(DateTimeOffset.UtcNow);
                    await PersistRunAsync(run, linkedCancel.Token);
                    await WriteAsync(writer, Info(run.Id, BackupRunStatus.Running, $"Backing up {source.DisplayName}."), cancellationToken);

                    var backup = RunBackupAsync(run, policy, environment, source, linkedCancel.Token);
                    await foreach (var item in backup.Stream)
                        await WriteAsync(writer, item, cancellationToken);

                    if (backup.Result.ExitCode != 0)
                    {
                        var status = backup.Result.ExitCode == -2 ? BackupRunStatus.TimedOut : BackupRunStatus.Failed;
                        var message = status == BackupRunStatus.TimedOut
                            ? $"Backup exceeded the {policy.TimeoutSeconds} second timeout."
                            : $"Restic backup exited with code {backup.Result.ExitCode}.";

                        await FailRunAsync(run, policy, status, backup.Result.ExitCode, ToErrorCode(status), message, CancellationToken.None);
                        await WriteAsync(writer, Error(run.Id, status, message, backup.Result.ExitCode), cancellationToken);
                        return;
                    }

                    if (string.IsNullOrWhiteSpace(backup.Result.SnapshotId))
                    {
                        const string message = "Restic completed without returning a snapshot ID.";
                        await FailRunAsync(run, policy, BackupRunStatus.Failed, backup.Result.ExitCode, "backup.snapshot_missing", message, CancellationToken.None);
                        await WriteAsync(writer, Error(run.Id, BackupRunStatus.Failed, message, backup.Result.ExitCode), cancellationToken);
                        return;
                    }

                    var warnings = new List<BackupRunWarning>();
                    if (policy.KeepLastSuccessful > 0)
                    {
                        run.MarkApplyingRetention();
                        await PersistRunAsync(run, CancellationToken.None);
                        await WriteAsync(writer, Info(run.Id, BackupRunStatus.ApplyingRetention, "Applying backup retention policy."), cancellationToken);

                        var retention = RunRetentionAsync(run, policy, environment, linkedCancel.Token);
                        await foreach (var item in retention.Stream)
                            await WriteAsync(writer, item, cancellationToken);

                        if (retention.Result.ExitCode != 0)
                        {
                            warnings.Add(new BackupRunWarning(
                                "backup.retention_failed",
                                $"Restic retention exited with code {retention.Result.ExitCode}."));
                        }
                    }

                    var completedAt = DateTimeOffset.UtcNow;
                    run.CompleteSucceeded(
                        backup.Result.SnapshotId!,
                        backup.Result.ParentSnapshotId,
                        backup.Result.FilesProcessed,
                        backup.Result.BytesProcessed,
                        backup.Result.BytesAdded,
                        warnings,
                        completedAt);

                    await CompleteRunAsync(run, policy, successful: true, completedAt, CancellationToken.None);
                    await WriteAsync(writer, Info(run.Id, run.Status, $"Backup \"{policy.Name}\" finished with status {run.Status}."), cancellationToken);
                }
            }
            catch (OperationCanceledException) when (timeoutCancel.IsCancellationRequested)
            {
                var message = $"Backup exceeded the {policy.TimeoutSeconds} second timeout.";
                await FailRunAsync(run, policy, BackupRunStatus.TimedOut, -2, "backup.timeout", message, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.TimedOut, message, -2), CancellationToken.None);
            }
            catch (OperationCanceledException) when (operationToken.IsCancellationRequested)
            {
                await CancelRunAsync(run, policy, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.Cancelled, "Backup run cancelled."), CancellationToken.None);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await CancelRunAsync(run, policy, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.Cancelled, "Backup run cancelled."), CancellationToken.None);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Backup run {RunId} failed.", run.Id);
                await FailRunAsync(run, policy, BackupRunStatus.Failed, null, "backup.failed", ex.Message, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRunStatus.Failed, ex.Message), CancellationToken.None);
            }
            finally
            {
                if (sourceLease)
                    await ReleaseSourceLeaseAsync(run, CancellationToken.None);

                await ReleaseRepositoryLeaseAsync(run, CancellationToken.None);
            }
        }
        finally
        {
            runCoordinator.Unregister(run.Id);
            writer.TryComplete();
        }
    }

    private async Task<BackupRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid runId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var plan = await uow.BackupRuns.TryClaimExecutionPlanAsync(runId, DateTimeOffset.UtcNow, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return plan;
    }

    private async Task<BackupRunExecutionPlan?> LoadExecutionPlanAsync(Guid runId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.BackupRuns.GetExecutionPlanAsync(runId, cancellationToken);
    }

    private async Task<Result<ResticRepositoryEnvironment>> BuildEnvironmentAsync(
        BackupRepository repository,
        BackupExecutionContext context,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await environmentBuilder.BuildAsync(uow, repository, context, cancellationToken);
    }

    private async Task<Result<BackupSourcePlan>> ResolveSourceAsync(BackupSourceSpec source, CancellationToken cancellationToken)
    {
        if (source is CitadelSystemBackupSource)
        {
            var path = Path.GetFullPath(options.CoreDataPath);
            Directory.CreateDirectory(path);
            return Result.Success(new BackupSourcePlan(
                path,
                "Citadel system data",
                new BackupExecutionContext(BackupExecutionLocation.Core, null),
                BuildSystemExcludes(path)));
        }

        if (source is DockerVolumeBackupSource volume)
        {
            if (volume.Consistency != VolumeBackupConsistency.Live)
                return Result.Failure<BackupSourcePlan>(new BadRequestError("Stopping attached containers for volume backups is not implemented yet."));

            if (!platformContainerCache.TryGetCacheEntry(volume.PlatformId, out var platform, out var error))
                return Result.Failure<BackupSourcePlan>(error);

            if (platform.ConnectorType != PlatformConnectorType.Local)
                return Result.Failure<BackupSourcePlan>(new BadRequestError("Docker volume backup execution on remote platforms is not implemented yet."));

            var connector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
            var result = await connector.InspectVolumeAsync(
                new InspectDockerVolumeCommand(platform.Address, volume.VolumeName),
                cancellationToken);
            if (!result.IsSuccess(out var dockerVolume, out var inspectError))
                return Result.Failure<BackupSourcePlan>(inspectError!);

            if (string.IsNullOrWhiteSpace(dockerVolume.Mountpoint))
                return Result.Failure<BackupSourcePlan>(new BadRequestError("Docker volume mountpoint is not available."));

            var path = Path.GetFullPath(dockerVolume.Mountpoint);
            if (!Directory.Exists(path))
                return Result.Failure<BackupSourcePlan>(new BadRequestError("Docker volume mountpoint does not exist on this host."));

            return Result.Success(new BackupSourcePlan(
                path,
                $"Docker volume {volume.VolumeName}",
                new BackupExecutionContext(BackupExecutionLocation.Core, null),
                []));
        }

        return Result.Failure<BackupSourcePlan>(new BadRequestError("Unsupported backup source type."));
    }

    private IReadOnlyList<string> BuildSystemExcludes(string sourcePath)
    {
        var excludes = new List<string>();
        AddExcludeIfInside(sourcePath, options.WorkingDirectory, excludes);

        foreach (var path in options.AllowedCorePaths.Where(static path => !string.IsNullOrWhiteSpace(path)))
            AddExcludeIfInside(sourcePath, path, excludes);

        return excludes;
    }

    private static void AddExcludeIfInside(string sourcePath, string candidatePath, ICollection<string> excludes)
    {
        var source = EnsureTrailingSeparator(Path.GetFullPath(sourcePath));
        var candidate = Path.GetFullPath(candidatePath);
        var normalized = EnsureTrailingSeparator(candidate);

        if (normalized.StartsWith(source, OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal)
            || string.Equals(source.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar), candidate, OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal))
        {
            excludes.Add(candidate);
        }
    }

    private BackupResticRun RunBackupAsync(
        BackupRun run,
        BackupPolicy policy,
        ResticRepositoryEnvironment environment,
        BackupSourcePlan source,
        CancellationToken cancellationToken)
    {
        var args = new List<string>(environment.CommonArguments)
        {
            "backup",
            "--json",
            "--tag",
            "citadel",
            "--tag",
            $"backup-policy:{policy.Id}",
            "--tag",
            $"backup-run:{run.Id}"
        };

        foreach (var exclude in source.ExcludePaths)
        {
            args.Add("--exclude");
            args.Add(exclude);
        }

        args.Add(source.Path);
        return RunResticAsync(run, environment, args, policy.TimeoutSeconds, cancellationToken);
    }

    private BackupResticRun RunRetentionAsync(
        BackupRun run,
        BackupPolicy policy,
        ResticRepositoryEnvironment environment,
        CancellationToken cancellationToken)
    {
        var args = new List<string>(environment.CommonArguments)
        {
            "forget",
            "--json",
            "--prune",
            "--keep-last",
            policy.KeepLastSuccessful.ToString(),
            "--tag",
            $"backup-policy:{policy.Id}"
        };

        return RunResticAsync(run, environment, args, policy.TimeoutSeconds, cancellationToken);
    }

    private BackupResticRun RunResticAsync(
        BackupRun run,
        ResticRepositoryEnvironment environment,
        IReadOnlyList<string> arguments,
        int timeoutSeconds,
        CancellationToken cancellationToken)
    {
        var result = new ResticBackupResult();

        async IAsyncEnumerable<BackupRunStreamItem> Stream([EnumeratorCancellation] CancellationToken ct)
        {
            var buffer = new BackupLogBudget(Math.Max(1024, options.MaxLogBytes));
            var pendingLogs = new List<BackupRunLogEntry>(LogBatchSize);
            try
            {
                await foreach (var item in processRunner.RunAsync(
                    new ResticProcessCommand(
                        options.ResticPath,
                        arguments,
                        environment.Environment,
                        environment.WorkingDirectory,
                        TimeSpan.FromSeconds(Math.Max(60, timeoutSeconds)),
                        environment.RedactionValues,
                        Math.Max(1024, options.MaxLogLineBytes)),
                    ct))
                {
                    if (item.ExitCode.HasValue)
                    {
                        result.ExitCode = item.ExitCode.Value;
                        yield return new BackupRunStreamItem(run.Id, null, null, ExitCode: item.ExitCode.Value);
                        continue;
                    }

                    if (item.Message is null)
                        continue;

                    var stream = item.Stream == ResticProcessStream.StdErr ? "stderr" : "stdout";
                    var message = buffer.Append(item.Message);
                    if (message is null)
                        continue;

                    pendingLogs.Add(new BackupRunLogEntry(Guid.CreateVersion7(), run.Id, DateTimeOffset.UtcNow, stream, message));
                    if (pendingLogs.Count >= LogBatchSize)
                        await FlushLogsAsync(pendingLogs, ct);

                    if (item.Stream == ResticProcessStream.StdOut)
                        UpdateSummary(message, result);

                    yield return new BackupRunStreamItem(run.Id, run.Status, message, stream);
                }
            }
            finally
            {
                await FlushLogsAsync(pendingLogs, CancellationToken.None);
            }
        }

        return new BackupResticRun(Stream(cancellationToken), result);
    }

    private async Task FlushLogsAsync(List<BackupRunLogEntry> entries, CancellationToken cancellationToken)
    {
        if (entries.Count == 0)
            return;

        var batch = entries.ToArray();
        entries.Clear();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRunLogs.AddRangeAsync(batch, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<bool> AcquireRepositoryLeaseAsync(BackupRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var now = DateTimeOffset.UtcNow;
        var result = await uow.BackupRepositoryLeases.TryAcquireForExistingRepositoryAsync(
            run.BackupRepositoryId,
            "Backup",
            run.Id,
            now.AddSeconds(Math.Max(30, options.RepositoryLeaseSeconds)),
            now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return result == BackupRepositoryLeaseAcquireResult.Acquired;
    }

    private async Task ReleaseRepositoryLeaseAsync(BackupRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositoryLeases.ReleaseAsync(run.BackupRepositoryId, run.Id, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<bool> AcquireSourceLeaseAsync(BackupRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var now = DateTimeOffset.UtcNow;
        var acquired = await uow.BackupSourceLeases.TryAcquireAsync(
            run.SourceSnapshot.StableKey,
            "Backup",
            run.Id,
            now.AddSeconds(Math.Max(30, options.SourceLeaseSeconds)),
            now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return acquired;
    }

    private async Task ReleaseSourceLeaseAsync(BackupRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupSourceLeases.ReleaseAsync(run.SourceSnapshot.StableKey, run.Id, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task PersistRunAsync(BackupRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRuns.UpdateAsync(run, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task CompleteRunAsync(BackupRun run, BackupPolicy? policy, bool successful, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        if (policy is null)
            await uow.BackupRuns.UpdateAsync(run, cancellationToken);
        else
            await uow.BackupRuns.FinishRunAndMarkPolicyIdleAsync(run, policy.Id, successful, completedAt, cancellationToken);

        await uow.CommitAsync(cancellationToken);
    }

    private async Task FailRunAsync(
        BackupRun run,
        BackupPolicy? policy,
        BackupRunStatus status,
        int? exitCode,
        string errorCode,
        string errorMessage,
        CancellationToken cancellationToken)
    {
        run.Fail(status, exitCode, errorCode, errorMessage, DateTimeOffset.UtcNow);
        await CompleteRunAsync(run, policy, successful: false, run.CompletedAt ?? DateTimeOffset.UtcNow, cancellationToken);
    }

    private async Task CancelRunAsync(BackupRun run, BackupPolicy? policy, CancellationToken cancellationToken)
    {
        try
        {
            run.Cancel(DateTimeOffset.UtcNow);
        }
        catch (InvalidOperationException)
        {
        }

        await CompleteRunAsync(run, policy, successful: false, run.CompletedAt ?? DateTimeOffset.UtcNow, cancellationToken);
    }

    private static void UpdateSummary(string line, ResticBackupResult result)
    {
        try
        {
            using var document = JsonDocument.Parse(line);
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
                return;

            var messageType = root.TryGetProperty("message_type", out var type)
                ? type.GetString()
                : null;
            if (!string.Equals(messageType, "summary", StringComparison.OrdinalIgnoreCase))
                return;

            result.SnapshotId = GetString(root, "snapshot_id") ?? result.SnapshotId;
            result.ParentSnapshotId = GetString(root, "parent_snapshot_id") ?? result.ParentSnapshotId;
            result.FilesProcessed = GetInt64(root, "total_files_processed") ?? result.FilesProcessed;
            result.BytesProcessed = GetInt64(root, "total_bytes_processed") ?? result.BytesProcessed;
            result.BytesAdded = GetInt64(root, "data_added") ?? result.BytesAdded;
        }
        catch (JsonException)
        {
        }
    }

    private static string? GetString(JsonElement element, string propertyName)
        => element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.String
            ? property.GetString()
            : null;

    private static long? GetInt64(JsonElement element, string propertyName)
    {
        if (!element.TryGetProperty(propertyName, out var property))
            return null;

        if (property.ValueKind == JsonValueKind.Number && property.TryGetInt64(out var value))
            return value;

        return null;
    }

    private static string ToErrorCode(BackupRunStatus status)
        => status switch
        {
            BackupRunStatus.TimedOut => "backup.timeout",
            BackupRunStatus.Rejected => "backup.rejected",
            BackupRunStatus.Cancelled => "backup.cancelled",
            _ => "backup.failed"
        };

    private static ValueTask WriteAsync(
        ChannelWriter<BackupRunStreamItem> writer,
        BackupRunStreamItem item,
        CancellationToken cancellationToken)
        => writer.WriteAsync(item, cancellationToken);

    private static BackupRunStreamItem Info(Guid runId, BackupRunStatus status, string message)
        => new(runId, status, message);

    private static BackupRunStreamItem Error(Guid runId, BackupRunStatus status, string message, int? exitCode = null)
        => new(runId, status, message, "stderr", exitCode);

    private static string EnsureTrailingSeparator(string path)
        => path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar;
}

internal sealed record BackupSourcePlan(
    string Path,
    string DisplayName,
    BackupExecutionContext Context,
    IReadOnlyList<string> ExcludePaths);

internal sealed record BackupResticRun(IAsyncEnumerable<BackupRunStreamItem> Stream, ResticBackupResult Result);

internal sealed class ResticBackupResult
{
    public int ExitCode { get; set; } = -1;
    public string? SnapshotId { get; set; }
    public string? ParentSnapshotId { get; set; }
    public long? FilesProcessed { get; set; }
    public long? BytesProcessed { get; set; }
    public long? BytesAdded { get; set; }
}

internal sealed class BackupLogBudget(int maxBytes)
{
    private int usedBytes;
    private bool exhausted;

    public string? Append(string value)
    {
        if (exhausted)
            return null;

        var bytes = Encoding.UTF8.GetByteCount(value);
        if (usedBytes + bytes <= maxBytes)
        {
            usedBytes += bytes;
            return value;
        }

        var remaining = maxBytes - usedBytes;
        exhausted = true;
        if (remaining <= 0)
            return "[log truncated]";

        var take = Math.Min(value.Length, remaining);
        usedBytes = maxBytes;
        return value[..take] + "... [log truncated]";
    }
}
