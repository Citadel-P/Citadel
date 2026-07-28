using Application.Configs;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Backups;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Threading.Channels;

namespace Application.Services.Backups;

public interface IBackupRestoreRunExecutionService
{
    IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteQueuedAsync(Guid restoreRunId, CancellationToken cancellationToken);
    IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteAsync(Guid restoreRunId, CancellationToken cancellationToken);
}

public interface IBackupRestoreRunCoordinator
{
    CancellationTokenSource Register(Guid restoreRunId);
    bool Cancel(Guid restoreRunId);
    void Unregister(Guid restoreRunId);
}

public sealed record BackupRestoreRunStreamItem(
    Guid RestoreRunId,
    BackupRestoreStatus? Status,
    string? Message,
    string? Stream = null,
    int? ExitCode = null);

internal sealed class BackupRestoreRunCoordinator : IBackupRestoreRunCoordinator
{
    private readonly ConcurrentDictionary<Guid, CancellationTokenSource> runs = new();

    public CancellationTokenSource Register(Guid restoreRunId)
    {
        var cts = new CancellationTokenSource();
        if (!runs.TryAdd(restoreRunId, cts))
        {
            cts.Dispose();
            throw new InvalidOperationException($"Backup restore run {restoreRunId} is already registered.");
        }

        return cts;
    }

    public bool Cancel(Guid restoreRunId)
    {
        if (!runs.TryGetValue(restoreRunId, out var cts))
            return false;

        cts.Cancel();
        return true;
    }

    public void Unregister(Guid restoreRunId)
    {
        if (runs.TryRemove(restoreRunId, out var cts))
            cts.Dispose();
    }
}

internal sealed class BackupRestoreRunExecutionService(
    IServiceScopeFactory scopeFactory,
    IResticEnvironmentBuilder environmentBuilder,
    IResticProcessRunner processRunner,
    IPlatformResticRunner platformResticRunner,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    IBackupRestoreRunCoordinator runCoordinator,
    IBackupRestoreRunStreamManager backupRestoreRunStreamManager,
    INotificationQueue notificationQueue,
    IOptions<BackupOptions> backupOptions,
    ILogger<BackupRestoreRunExecutionService> logger) : IBackupRestoreRunExecutionService
{
    private const int LogBatchSize = 25;
    private const string ResticExecutable = "restic";
    private const string PlatformSourceMountPath = "/source";
    private const string PlatformTargetMountPath = "/target";
    private readonly BackupOptions options = backupOptions.Value;

    public async IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteQueuedAsync(
        Guid restoreRunId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var plan = await TryClaimExecutionPlanAsync(restoreRunId, cancellationToken);
        if (plan is null)
        {
            yield return Error(restoreRunId, BackupRestoreStatus.Rejected, "Backup restore run is no longer queued.");
            yield break;
        }

        await NotifyBackupRestoreRunAsync(plan.RestoreRun, plan.BackupRun.BackupPolicyId, cancellationToken);

        await foreach (var item in ExecutePlanAsync(plan, cancellationToken))
            yield return item;
    }

    public async IAsyncEnumerable<BackupRestoreRunStreamItem> ExecuteAsync(
        Guid restoreRunId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var plan = await LoadExecutionPlanAsync(restoreRunId, cancellationToken);
        if (plan is null)
            yield break;

        await foreach (var item in ExecutePlanAsync(plan, cancellationToken))
            yield return item;
    }

    private async IAsyncEnumerable<BackupRestoreRunStreamItem> ExecutePlanAsync(
        BackupRestoreRunExecutionPlan plan,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var channel = Channel.CreateBounded<BackupRestoreRunStreamItem>(
            new BoundedChannelOptions(256)
            {
                SingleReader = true,
                SingleWriter = true,
                FullMode = BoundedChannelFullMode.Wait
            });
        using var producerCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);

        var producer = RunExecutionProducerAsync(
            plan,
            channel.Writer,
            producerCts.Token);

        try
        {
            await foreach (var item in channel.Reader.ReadAllAsync(producerCts.Token))
                yield return item;

            await producer;
        }
        finally
        {
            await producerCts.CancelAsync();
            channel.Writer.TryComplete();
            await ObserveProducerAsync(producer);
        }
    }

    private static async Task ObserveProducerAsync(Task producer)
    {
        try
        {
            await producer;
        }
        catch
        {
        }
    }

    private async Task RunExecutionProducerAsync(
        BackupRestoreRunExecutionPlan plan,
        ChannelWriter<BackupRestoreRunStreamItem> writer,
        CancellationToken cancellationToken)
    {
        try
        {
            await ExecuteCoreAsync(plan, writer, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Backup restore run {RestoreRunId} failed before the execution service completed.", plan.RestoreRun.Id);
            writer.TryComplete(ex);
        }
    }

    private async Task ExecuteCoreAsync(
        BackupRestoreRunExecutionPlan context,
        ChannelWriter<BackupRestoreRunStreamItem> writer,
        CancellationToken cancellationToken)
    {
        var run = context.RestoreRun;
        var backupRun = context.BackupRun;
        var backupPolicyId = backupRun.BackupPolicyId;
        var repository = context.Repository;
        var operationToken = runCoordinator.Register(run.Id);

        try
        {
            using var timeoutCancel = new CancellationTokenSource(TimeSpan.FromSeconds(Math.Max(60, options.DefaultTimeoutSeconds)));
            using var linkedCancel = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCancel.Token, operationToken.Token);

            await WriteAsync(writer, Info(run.Id, BackupRestoreStatus.Preparing, $"Restore for snapshot \"{backupRun.ResticSnapshotId}\" is preparing."), cancellationToken);

            if (!options.Enabled)
            {
                await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.disabled", "Backups are disabled.", cancellationToken);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, "Backups are disabled."), cancellationToken);
                return;
            }

            if (backupRun.SnapshotAvailability != BackupSnapshotAvailability.Available || string.IsNullOrWhiteSpace(backupRun.ResticSnapshotId))
            {
                const string message = "Backup snapshot is not available for restore.";
                await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.snapshot_unavailable", message, cancellationToken);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                return;
            }

            var repositoryLease = await AcquireRepositoryLeaseAsync(run, linkedCancel.Token);
            if (!repositoryLease)
            {
                const string message = "Backup repository already has an active operation.";
                await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.repository_busy", message, cancellationToken);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                return;
            }

            var targetLeaseKey = $"{run.TargetPlatformId}:{run.TargetVolumeName}";
            var sourceLease = false;
            try
            {
                sourceLease = await AcquireSourceLeaseAsync(run, targetLeaseKey, linkedCancel.Token);
                if (!sourceLease)
                {
                    const string message = "Backup restore target already has an active operation.";
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.target_busy", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                    return;
                }

                if (!platformContainerCache.TryGetCacheEntry(run.TargetPlatformId, out var targetPlatform, out var targetPlatformError))
                {
                    var message = targetPlatformError.Message;
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.target_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                    return;
                }

                var repositoryContext = ResolveRestoreExecutionContext(repository, targetPlatform);
                if (!repositoryContext.IsSuccess(out var executionContext, out var contextError))
                {
                    var message = contextError?.Message ?? "Backup repository cannot restore to the target platform.";
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.repository_location_unsupported", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                    return;
                }

                var sourceResult = await ResolveSourceSnapshotAsync(backupRun, repository, linkedCancel.Token);
                if (!sourceResult.IsSuccess(out var source, out var sourceError))
                {
                    var message = sourceError?.Message ?? "Backup source could not be resolved.";
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.source_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                    return;
                }

                var targetResult = await PrepareTargetVolumeAsync(run, targetPlatform, executionContext, linkedCancel.Token);
                if (!targetResult.IsSuccess(out var target, out var targetError))
                {
                    var message = targetError?.Message ?? "Backup restore target could not be prepared.";
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Rejected, null, "backup.restore.target_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Rejected, message), cancellationToken);
                    return;
                }

                var environmentResult = await BuildEnvironmentAsync(repository, executionContext, linkedCancel.Token);
                if (!environmentResult.IsSuccess(out var environment, out var environmentError))
                {
                    var message = environmentError?.Message ?? "Backup repository could not be prepared.";
                    await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Failed, null, "backup.repository_unavailable", message, cancellationToken);
                    await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Failed, message), cancellationToken);
                    return;
                }

                await using (environment)
                {
                    run.MarkRunning(DateTimeOffset.UtcNow);
                    await PersistRunAsync(run, backupPolicyId, linkedCancel.Token);
                    await WriteAsync(writer, Info(run.Id, BackupRestoreStatus.Running, $"Restoring snapshot into volume \"{run.TargetVolumeName}\"."), cancellationToken);

                    var restore = RunRestoreAsync(run, backupRun.ResticSnapshotId!, environment, source, target, linkedCancel.Token);
                    await foreach (var item in restore.Stream)
                        await WriteAsync(writer, item, cancellationToken);

                    if (restore.Result.ExitCode != 0)
                    {
                        var status = restore.Result.ExitCode == -2 ? BackupRestoreStatus.TimedOut : BackupRestoreStatus.Failed;
                        var message = status == BackupRestoreStatus.TimedOut
                            ? $"Restore exceeded the {options.DefaultTimeoutSeconds} second timeout."
                            : $"Restic restore exited with code {restore.Result.ExitCode}.";

                        await FailRunAsync(run, backupPolicyId, status, restore.Result.ExitCode, ToErrorCode(status), message, CancellationToken.None);
                        await WriteAsync(writer, Error(run.Id, status, message, restore.Result.ExitCode), cancellationToken);
                        return;
                    }

                    var completedAt = DateTimeOffset.UtcNow;
                    run.CompleteSucceeded(target.CreatedByCitadel, [], [], completedAt);
                    await CompleteRunAsync(run, backupPolicyId, completedAt, CancellationToken.None);
                    await WriteAsync(writer, Info(run.Id, run.Status, $"Restore finished with status {run.Status}."), cancellationToken);
                }
            }
            catch (OperationCanceledException) when (timeoutCancel.IsCancellationRequested)
            {
                var message = $"Restore exceeded the {options.DefaultTimeoutSeconds} second timeout.";
                await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.TimedOut, -2, "backup.restore.timeout", message, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.TimedOut, message, -2), CancellationToken.None);
            }
            catch (OperationCanceledException) when (operationToken.IsCancellationRequested)
            {
                await CancelRunAsync(run, backupPolicyId, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Cancelled, "Backup restore run cancelled."), CancellationToken.None);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await CancelRunAsync(run, backupPolicyId, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Cancelled, "Backup restore run cancelled."), CancellationToken.None);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Backup restore run {RestoreRunId} failed.", run.Id);
                await FailRunAsync(run, backupPolicyId, BackupRestoreStatus.Failed, null, "backup.restore.failed", ex.Message, CancellationToken.None);
                await WriteAsync(writer, Error(run.Id, BackupRestoreStatus.Failed, ex.Message), CancellationToken.None);
            }
            finally
            {
                if (sourceLease)
                    await ReleaseSourceLeaseAsync(targetLeaseKey, run.Id, CancellationToken.None);

                await ReleaseRepositoryLeaseAsync(run, CancellationToken.None);
            }
        }
        finally
        {
            runCoordinator.Unregister(run.Id);
            writer.TryComplete();
        }
    }

    private async Task<BackupRestoreRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid restoreRunId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var plan = await uow.BackupRestoreRuns.TryClaimExecutionPlanAsync(restoreRunId, DateTimeOffset.UtcNow, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return plan;
    }

    private async Task<BackupRestoreRunExecutionPlan?> LoadExecutionPlanAsync(Guid restoreRunId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.BackupRestoreRuns.GetExecutionPlanAsync(restoreRunId, cancellationToken);
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

    private async Task<Result<BackupRestoreSourcePlan>> ResolveSourceSnapshotAsync(
        BackupRun backupRun,
        BackupRepository repository,
        CancellationToken cancellationToken)
    {
        if (backupRun.SourceSnapshot is not DockerVolumeBackupSource source)
            return Result.Failure<BackupRestoreSourcePlan>(new BadRequestError("Only Docker volume backup snapshots can be restored to Docker volumes."));

        if (!platformContainerCache.TryGetCacheEntry(source.PlatformId, out var platform, out var cacheError))
            return Result.Failure<BackupRestoreSourcePlan>(cacheError);

        var backupContext = ResolveBackupExecutionContext(repository, platform);
        if (!backupContext.IsSuccess(out var executionContext, out var contextError))
            return Result.Failure<BackupRestoreSourcePlan>(contextError!);

        if (executionContext.Location == BackupExecutionLocation.Platform)
            return Result.Success(new BackupRestoreSourcePlan(PlatformSourceMountPath));

        var connector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
        var volumeResult = await connector.InspectVolumeAsync(
            new InspectDockerVolumeCommand(platform.Address, source.VolumeName),
            cancellationToken);
        if (!volumeResult.IsSuccess(out var volume, out var inspectError))
            return Result.Failure<BackupRestoreSourcePlan>(inspectError!);

        if (string.IsNullOrWhiteSpace(volume.Mountpoint))
            return Result.Failure<BackupRestoreSourcePlan>(new BadRequestError("Backup source volume mountpoint is not available."));

        if (!LocalDockerVolumePathResolver.TryResolve(
                volume.Mountpoint,
                out var path))
        {
            return Result.Failure<BackupRestoreSourcePlan>(
                new BadRequestError(
                    "Backup source volume mountpoint is not accessible to Citadel Core."));
        }

        return Result.Success(new BackupRestoreSourcePlan(path));
    }

    private async Task<Result<BackupRestoreTargetPlan>> PrepareTargetVolumeAsync(
        BackupRestoreRun run,
        PlatformCacheEntry platform,
        BackupExecutionContext executionContext,
        CancellationToken cancellationToken)
    {
        var connector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
        var listResult = await connector.ListVolumesAsync(
            new ListdDockerVolumesCommand(platform.Address, Dangling: null, Driver: null, Name: run.TargetVolumeName),
            cancellationToken);
        if (!listResult.IsSuccess(out var volumes, out var listError))
            return Result.Failure<BackupRestoreTargetPlan>(listError!);

        var existing = volumes.FirstOrDefault(volume => string.Equals(volume.Name, run.TargetVolumeName, StringComparison.Ordinal));
        if (existing is not null)
        {
            if (!run.OverwriteExisting)
                return Result.Failure<BackupRestoreTargetPlan>(new ConflictError("Target volume already exists. Enable overwrite to replace it."));

            if (existing.InUse || existing.Containers.Any())
                return Result.Failure<BackupRestoreTargetPlan>(new ConflictError("Target volume is in use and cannot be overwritten."));

            var deleteResult = await connector.DeleteVolumeAsync(
                new DeleteDockerVolumeCommand(platform.Address, Force: false, [run.TargetVolumeName]),
                cancellationToken);
            if (!deleteResult.IsSuccess())
                return Result.Failure<BackupRestoreTargetPlan>(deleteResult.Errors);
        }

        var labels = new Dictionary<string, string>
        {
            ["citadel.backup.restoreRunId"] = run.Id.ToString()
        };
        var createResult = await connector.CreateVolumeAsync(
            new CreateDockerVolumeCommand(platform.Address, run.TargetVolumeName, "local", labels, new Dictionary<string, string>()),
            cancellationToken);
        if (!createResult.IsSuccess(out var created, out var createError))
            return Result.Failure<BackupRestoreTargetPlan>(createError!);

        if (executionContext.Location == BackupExecutionLocation.Platform)
            return Result.Success(new BackupRestoreTargetPlan(PlatformTargetMountPath, CreatedByCitadel: true));

        if (string.IsNullOrWhiteSpace(created.Mountpoint))
            return Result.Failure<BackupRestoreTargetPlan>(new BadRequestError("Target volume mountpoint is not available."));

        if (!LocalDockerVolumePathResolver.TryResolve(
                created.Mountpoint,
                out var path))
        {
            return Result.Failure<BackupRestoreTargetPlan>(
                new BadRequestError(
                    "Target volume mountpoint is not accessible to Citadel Core."));
        }

        return Result.Success(new BackupRestoreTargetPlan(path, CreatedByCitadel: true));
    }

    private BackupRestoreResticRun RunRestoreAsync(
        BackupRestoreRun run,
        string snapshotId,
        ResticRepositoryEnvironment environment,
        BackupRestoreSourcePlan source,
        BackupRestoreTargetPlan target,
        CancellationToken cancellationToken)
    {
        var args = new List<string>(environment.CommonArguments)
        {
            "restore",
            $"{snapshotId}:{source.Path}",
            "--target",
            target.Path
        };

        return RunResticAsync(run, environment, args, cancellationToken);
    }

    private BackupRestoreResticRun RunResticAsync(
        BackupRestoreRun run,
        ResticRepositoryEnvironment environment,
        IReadOnlyList<string> arguments,
        CancellationToken cancellationToken)
    {
        var result = new BackupRestoreResticResult();

        async IAsyncEnumerable<BackupRestoreRunStreamItem> Stream([EnumeratorCancellation] CancellationToken ct)
        {
            var buffer = new BackupLogBudget(Math.Max(1024, options.MaxLogBytes));
            var pendingLogs = new List<BackupRestoreRunLogEntry>(LogBatchSize);
            try
            {
                await foreach (var item in BuildResticStream(run, environment, arguments, ct))
                {
                    if (item.ExitCode.HasValue)
                    {
                        result.ExitCode = item.ExitCode.Value;
                        yield return new BackupRestoreRunStreamItem(run.Id, null, null, ExitCode: item.ExitCode.Value);
                        continue;
                    }

                    if (item.Message is null)
                        continue;

                    var stream = item.Stream == ResticProcessStream.StdErr ? "stderr" : "stdout";
                    var message = buffer.Append(item.Message);
                    if (message is null)
                        continue;

                    pendingLogs.Add(new BackupRestoreRunLogEntry(Guid.CreateVersion7(), run.Id, DateTimeOffset.UtcNow, stream, message));
                    if (pendingLogs.Count >= LogBatchSize)
                        await FlushLogsAsync(pendingLogs, ct);

                    yield return new BackupRestoreRunStreamItem(run.Id, run.Status, message, stream);
                }
            }
            finally
            {
                await FlushLogsAsync(pendingLogs, CancellationToken.None);
            }
        }

        return new BackupRestoreResticRun(Stream(cancellationToken), result);
    }

    private IAsyncEnumerable<ResticProcessEvent> BuildResticStream(
        BackupRestoreRun run,
        ResticRepositoryEnvironment environment,
        IReadOnlyList<string> arguments,
        CancellationToken cancellationToken)
    {
        var timeout = TimeSpan.FromSeconds(Math.Max(60, options.DefaultTimeoutSeconds));
        if (environment.Context.Location == BackupExecutionLocation.Core)
        {
            return processRunner.RunAsync(
                new ResticProcessCommand(
                    options.ResticPath,
                    arguments,
                    environment.Environment,
                    environment.WorkingDirectory,
                    timeout,
                    environment.RedactionValues,
                    Math.Max(1024, options.MaxLogLineBytes)),
                cancellationToken);
        }

        if (!environment.Context.PlatformId.HasValue)
            return SingleResticError("Platform restore execution requires a platform ID.");

        if (!platformContainerCache.TryGetCacheEntry(environment.Context.PlatformId.Value, out var platform, out var cacheError))
            return SingleResticError(cacheError.Message);

        return platformResticRunner.RunAsync(
            new PlatformResticCommand(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                ResticExecutable,
                arguments,
                environment.RemoteEnvironment,
                timeout,
                environment.RedactionValues,
                Math.Max(1024, options.MaxLogLineBytes),
                SourceVolumeName: null,
                TargetVolumeName: run.TargetVolumeName,
                RepositoryHostPath: environment.PlatformRepositoryHostPath,
                NetworkMode: environment.RepositoryRequiresNetwork ? null : "none"),
            cancellationToken);
    }

    private static async IAsyncEnumerable<ResticProcessEvent> SingleResticError(string message)
    {
        await Task.Yield();
        yield return new ResticProcessEvent(ResticProcessStream.StdErr, message);
        yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
    }

    private async Task FlushLogsAsync(List<BackupRestoreRunLogEntry> entries, CancellationToken cancellationToken)
    {
        if (entries.Count == 0)
            return;

        var batch = entries.ToArray();
        entries.Clear();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRestoreRunLogs.AddRangeAsync(batch, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<bool> AcquireRepositoryLeaseAsync(BackupRestoreRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var now = DateTimeOffset.UtcNow;
        var result = await uow.BackupRepositoryLeases.TryAcquireForExistingRepositoryAsync(
            run.BackupRepositoryId,
            "Restore",
            run.Id,
            now.AddSeconds(Math.Max(30, options.RepositoryLeaseSeconds)),
            now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return result == BackupRepositoryLeaseAcquireResult.Acquired;
    }

    private async Task ReleaseRepositoryLeaseAsync(BackupRestoreRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositoryLeases.ReleaseAsync(run.BackupRepositoryId, run.Id, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<bool> AcquireSourceLeaseAsync(BackupRestoreRun run, string sourceKey, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var now = DateTimeOffset.UtcNow;
        var acquired = await uow.BackupSourceLeases.TryAcquireAsync(
            sourceKey,
            "Restore",
            run.Id,
            now.AddSeconds(Math.Max(30, options.SourceLeaseSeconds)),
            now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);
        return acquired;
    }

    private async Task ReleaseSourceLeaseAsync(string sourceKey, Guid restoreRunId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupSourceLeases.ReleaseAsync(sourceKey, restoreRunId, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task PersistRunAsync(BackupRestoreRun run, Guid backupPolicyId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRestoreRuns.UpdateAsync(run, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        await NotifyBackupRestoreRunAsync(run, backupPolicyId, cancellationToken);
    }

    private async Task CompleteRunAsync(BackupRestoreRun run, Guid backupPolicyId, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRestoreRuns.FinishRunAsync(run, completedAt, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        await NotifyBackupRestoreRunAsync(run, backupPolicyId, cancellationToken);
    }

    private ValueTask NotifyBackupRestoreRunAsync(BackupRestoreRun run, Guid backupPolicyId, CancellationToken cancellationToken)
        => notificationQueue.EnqueueAsync(
            new BackupRestoreRunNotificationWorkItem(backupRestoreRunStreamManager, run, backupPolicyId),
            cancellationToken);

    private async Task FailRunAsync(
        BackupRestoreRun run,
        Guid backupPolicyId,
        BackupRestoreStatus status,
        int? exitCode,
        string errorCode,
        string errorMessage,
        CancellationToken cancellationToken)
    {
        run.Fail(status, exitCode, errorCode, errorMessage, DateTimeOffset.UtcNow);
        await CompleteRunAsync(run, backupPolicyId, run.CompletedAt ?? DateTimeOffset.UtcNow, cancellationToken);
    }

    private async Task CancelRunAsync(BackupRestoreRun run, Guid backupPolicyId, CancellationToken cancellationToken)
    {
        try
        {
            run.Cancel(DateTimeOffset.UtcNow);
        }
        catch (InvalidOperationException)
        {
        }

        await CompleteRunAsync(run, backupPolicyId, run.CompletedAt ?? DateTimeOffset.UtcNow, cancellationToken);
    }

    private static Result<BackupExecutionContext> ResolveRestoreExecutionContext(
        BackupRepository repository,
        PlatformCacheEntry targetPlatform)
    {
        if (repository.Spec is FileSystemBackupRepositorySpec fs)
        {
            if (fs.Location == BackupExecutionLocation.Core)
            {
                return targetPlatform.ConnectorType == PlatformConnectorType.Local
                    ? new BackupExecutionContext(BackupExecutionLocation.Core, null)
                    : Result.Failure<BackupExecutionContext>(new BadRequestError("Core filesystem backup repositories cannot restore to remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform."));
            }

            if (fs.PlatformId != targetPlatform.Id)
                return Result.Failure<BackupExecutionContext>(new BadRequestError("Filesystem backup repository platform must match the restore target platform."));

            return new BackupExecutionContext(BackupExecutionLocation.Platform, targetPlatform.Id);
        }

        if (repository.Spec is S3CompatibleBackupRepositorySpec)
        {
            return new BackupExecutionContext(BackupExecutionLocation.Platform, targetPlatform.Id);
        }

        return Result.Failure<BackupExecutionContext>(new BadRequestError("Unsupported backup repository type."));
    }

    private static Result<BackupExecutionContext> ResolveBackupExecutionContext(
        BackupRepository repository,
        PlatformCacheEntry sourcePlatform)
    {
        if (repository.Spec is FileSystemBackupRepositorySpec fs)
        {
            if (fs.Location == BackupExecutionLocation.Core)
            {
                return sourcePlatform.ConnectorType == PlatformConnectorType.Local
                    ? new BackupExecutionContext(BackupExecutionLocation.Core, null)
                    : Result.Failure<BackupExecutionContext>(new BadRequestError("Core filesystem backup repositories cannot contain remote Docker volume backups."));
            }

            if (fs.PlatformId != sourcePlatform.Id)
                return Result.Failure<BackupExecutionContext>(new BadRequestError("Filesystem backup repository platform must match the backup source platform."));

            return new BackupExecutionContext(BackupExecutionLocation.Platform, sourcePlatform.Id);
        }

        if (repository.Spec is S3CompatibleBackupRepositorySpec)
        {
            return new BackupExecutionContext(BackupExecutionLocation.Platform, sourcePlatform.Id);
        }

        return Result.Failure<BackupExecutionContext>(new BadRequestError("Unsupported backup repository type."));
    }

    private static string ToErrorCode(BackupRestoreStatus status)
        => status switch
        {
            BackupRestoreStatus.TimedOut => "backup.restore.timeout",
            BackupRestoreStatus.Rejected => "backup.restore.rejected",
            BackupRestoreStatus.Cancelled => "backup.restore.cancelled",
            _ => "backup.restore.failed"
        };

    private static ValueTask WriteAsync(
        ChannelWriter<BackupRestoreRunStreamItem> writer,
        BackupRestoreRunStreamItem item,
        CancellationToken cancellationToken)
        => writer.WriteAsync(item, cancellationToken);

    private static BackupRestoreRunStreamItem Info(Guid restoreRunId, BackupRestoreStatus status, string message)
        => new(restoreRunId, status, message);

    private static BackupRestoreRunStreamItem Error(Guid restoreRunId, BackupRestoreStatus status, string message, int? exitCode = null)
        => new(restoreRunId, status, message, "stderr", exitCode);
}

internal sealed record BackupRestoreSourcePlan(string Path);

internal sealed record BackupRestoreTargetPlan(string Path, bool CreatedByCitadel);

internal sealed record BackupRestoreResticRun(IAsyncEnumerable<BackupRestoreRunStreamItem> Stream, BackupRestoreResticResult Result);

internal sealed class BackupRestoreResticResult
{
    public int ExitCode { get; set; } = -1;
}
