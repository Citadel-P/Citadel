using Application.Configs;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Backups;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using System.Collections.Immutable;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Backups;

public sealed class BackupRunExecutionTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeResticProcessRunner restic = new();
    private readonly FakeVolumeConnector volumeConnector = new();
    private readonly FakeContainerConnector containerConnector = new();
    private readonly FakeImageConnector imageConnector = new();
    private readonly string testRoot = Path.Combine(Path.GetTempPath(), $"citadel-backup-runs-test-{Guid.NewGuid():N}");

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<IResticProcessRunner>(restic);
        services.ReplaceService<IConnectorFactory<IVolumeConnector>>(new FakeConnectorFactory<IVolumeConnector>(volumeConnector));
        services.ReplaceService<IConnectorFactory<IContainerConnector>>(new FakeConnectorFactory<IContainerConnector>(containerConnector));
        services.ReplaceService<IConnectorFactory<IImageConnector>>(new FakeConnectorFactory<IImageConnector>(imageConnector));
        services.Configure<BackupOptions>(options =>
        {
            options.Enabled = true;
            options.ResticPath = "restic-test";
            options.CoreDataPath = Path.Combine(testRoot, "data");
            options.WorkingDirectory = Path.Combine(testRoot, "data", "backups", "work");
            options.AllowedCorePaths = [Path.Combine(testRoot, "data", "backups", "repositories")];
            options.RepositoryLeaseSeconds = 60;
            options.SourceLeaseSeconds = 60;
            options.DefaultTimeoutSeconds = 120;
            options.MaxLogLineBytes = 4096;
            options.MaxLogBytes = 64 * 1024;
        });
    }

    [Fact]
    public async Task ExecuteQueuedAsync_ShouldRunRemoteDockerVolumeBackupThroughPlatformHelper()
    {
        var platformId = Guid.CreateVersion7();
        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platformId,
            new PlatformCacheEntry(platformId, "agent://platform-01", PlatformConnectorType.Agent, ImmutableDictionary<string, Guid>.Empty));

        volumeConnector.Volume = new DockerVolumeResult(
            Id: "remote-data",
            Name: "remote-data",
            InUse: true,
            Scope: "local",
            Driver: "local",
            Mountpoint: string.Empty,
            CreatedAt: DateTimeOffset.UtcNow.ToString("O"),
            ClusterVolume: null,
            UsageData: null,
            Containers: [],
            Status: new Dictionary<string, string>(),
            Labels: new Dictionary<string, string>(),
            Options: new Dictionary<string, string>());

        containerConnector.EnqueueExec(
            exitCode: 0,
            stdout: """
                {"message_type":"summary","snapshot_id":"remote-snapshot","total_files_processed":2,"total_bytes_processed":512,"data_added":128}
                """);

        var setup = await CreateRemoteVolumeRepositoryPolicyAndRunAsync(platformId, keepLastSuccessful: 0);

        var service = Services.GetRequiredService<IBackupRunExecutionService>();
        await foreach (var _ in service.ExecuteQueuedAsync(setup.Run.Id, TestContext.Current.CancellationToken))
        {
        }

        Assert.Empty(restic.Calls);

        var create = Assert.Single(containerConnector.CreateCommands);
        Assert.Equal("agent://platform-01", create.PlatformAddress);
        Assert.Contains(create.Mounts ?? [], mount =>
            mount.Type == "volume"
            && mount.Source == "remote-data"
            && mount.Target == "/source"
            && mount.ReadOnly == true);
        Assert.Null(create.NetworkMode);

        var exec = Assert.Single(containerConnector.ExecRequests);
        Assert.Equal("restic", exec.Request.Command[0]);
        Assert.Contains("backup", exec.Request.Command);
        Assert.Contains("/source", exec.Request.Command);
        Assert.Equal("restic-password", exec.Request.Environment?["RESTIC_PASSWORD"]);
        Assert.Equal("access-key", exec.Request.Environment?["AWS_ACCESS_KEY_ID"]);
        Assert.False(exec.Request.Environment?.ContainsKey("RESTIC_PASSWORD_FILE") ?? true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRuns.GetAsync(setup.Run.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRunStatus.Succeeded, storedRun.Status);
        Assert.Equal("remote-snapshot", storedRun.ResticSnapshotId);
    }

    [Fact]
    public async Task ExecuteQueuedAsync_ShouldCompleteRunApplyRetentionAndPersistLogs()
    {
        Directory.CreateDirectory(Path.Combine(testRoot, "data"));
        await File.WriteAllTextAsync(
            Path.Combine(testRoot, "data", "appsettings.json"),
            "{}",
            TestContext.Current.CancellationToken);

        var setup = await CreateRepositoryPolicyAndRunAsync("backup-success", keepLastSuccessful: 2);
        restic.Enqueue(
            exitCode: 0,
            stdout: """
                {"message_type":"summary","snapshot_id":"snapshot-001","total_files_processed":3,"total_bytes_processed":128,"data_added":64}
                """);
        restic.Enqueue(exitCode: 0, stdout: "[]");

        var service = Services.GetRequiredService<IBackupRunExecutionService>();
        var items = new List<BackupRunStreamItem>();
        await foreach (var item in service.ExecuteQueuedAsync(setup.Run.Id, TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.Status == BackupRunStatus.Running);
        Assert.Contains(items, item => item.Status == BackupRunStatus.ApplyingRetention);
        Assert.Contains(items, item => item.Status == BackupRunStatus.Succeeded);

        Assert.Equal(2, restic.Calls.Count);
        Assert.Contains("backup", restic.Calls[0].Command.Arguments);
        Assert.Contains(Path.GetFullPath(Path.Combine(testRoot, "data")), restic.Calls[0].Command.Arguments);
        Assert.Contains("--exclude", restic.Calls[0].Command.Arguments);
        Assert.Contains(Path.GetFullPath(Path.Combine(testRoot, "data", "backups", "repositories")), restic.Calls[0].Command.Arguments);
        Assert.Equal("forget", restic.Calls[1].Command.Arguments[0]);
        Assert.Contains("--keep-last", restic.Calls[1].Command.Arguments);
        Assert.Contains("2", restic.Calls[1].Command.Arguments);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRuns.GetAsync(setup.Run.Id, TestContext.Current.CancellationToken);
        var storedPolicy = await uow.BackupPolicies.GetAsync(setup.Policy.Id, TestContext.Current.CancellationToken);
        var logs = await uow.BackupRunLogs.GetByRunAsync(setup.Run.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRunStatus.Succeeded, storedRun.Status);
        Assert.Equal("snapshot-001", storedRun.ResticSnapshotId);
        Assert.Equal(3, storedRun.FilesProcessed);
        Assert.Equal(128, storedRun.BytesProcessed);
        Assert.Equal(64, storedRun.BytesAdded);
        Assert.Equal(BackupSnapshotAvailability.Available, storedRun.SnapshotAvailability);

        Assert.NotNull(storedPolicy);
        Assert.Equal(ResourceControlState.Idle, storedPolicy.ControlState);
        Assert.Null(storedPolicy.CurrentRunId);
        Assert.NotNull(storedPolicy.FirstSuccessfulRunAt);

        Assert.Contains(logs, log => log.Message.Contains("snapshot-001", StringComparison.Ordinal));

        var repositoryLeaseAvailable = await uow.BackupRepositoryLeases.TryAcquireAsync(
            setup.Repository.Id,
            "Test",
            Guid.CreateVersion7(),
            DateTimeOffset.UtcNow.AddMinutes(1),
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        var sourceLeaseAvailable = await uow.BackupSourceLeases.TryAcquireAsync(
            "citadel-system",
            "Test",
            Guid.CreateVersion7(),
            DateTimeOffset.UtcNow.AddMinutes(1),
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        Assert.True(repositoryLeaseAvailable);
        Assert.True(sourceLeaseAvailable);
    }

    [Fact]
    public async Task ExecuteQueuedAsync_ShouldFailRunAndPersistStderr()
    {
        Directory.CreateDirectory(Path.Combine(testRoot, "data"));
        var setup = await CreateRepositoryPolicyAndRunAsync("backup-failure", keepLastSuccessful: 2);
        restic.Enqueue(exitCode: 1, stderr: "repository is unavailable");

        var service = Services.GetRequiredService<IBackupRunExecutionService>();
        await foreach (var _ in service.ExecuteQueuedAsync(setup.Run.Id, TestContext.Current.CancellationToken))
        {
        }

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRuns.GetAsync(setup.Run.Id, TestContext.Current.CancellationToken);
        var storedPolicy = await uow.BackupPolicies.GetAsync(setup.Policy.Id, TestContext.Current.CancellationToken);
        var logs = await uow.BackupRunLogs.GetByRunAsync(setup.Run.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRunStatus.Failed, storedRun.Status);
        Assert.Equal(1, storedRun.ExitCode);
        Assert.Equal(BackupSnapshotAvailability.NotCreated, storedRun.SnapshotAvailability);
        Assert.Contains("Restic backup exited with code 1", storedRun.ErrorMessage);
        Assert.Contains(logs, log => log.Stream == "stderr" && log.Message == "repository is unavailable");

        Assert.NotNull(storedPolicy);
        Assert.Equal(ResourceControlState.Idle, storedPolicy.ControlState);
        Assert.Null(storedPolicy.CurrentRunId);
        Assert.Null(storedPolicy.FirstSuccessfulRunAt);
    }

    [Fact]
    public async Task ExecuteQueuedAsync_ShouldSucceedWithWarningWhenResticCreatesNoSnapshot()
    {
        Directory.CreateDirectory(Path.Combine(testRoot, "data"));
        var setup = await CreateRepositoryPolicyAndRunAsync("backup-empty", keepLastSuccessful: 1);
        restic.Enqueue(exitCode: 0);
        restic.Enqueue(exitCode: 0);

        var service = Services.GetRequiredService<IBackupRunExecutionService>();
        var items = new List<BackupRunStreamItem>();
        await foreach (var item in service.ExecuteQueuedAsync(setup.Run.Id, TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRuns.GetAsync(setup.Run.Id, TestContext.Current.CancellationToken);
        var logs = await uow.BackupRunLogs.GetByRunAsync(setup.Run.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRunStatus.SucceededWithWarnings, storedRun.Status);
        Assert.Equal(BackupSnapshotAvailability.NotCreated, storedRun.SnapshotAvailability);
        Assert.Null(storedRun.ResticSnapshotId);
        Assert.Null(storedRun.ErrorMessage);
        Assert.Contains(storedRun.Warnings, warning => warning.Code == "backup.snapshot_not_created");
        Assert.Contains(items, item => item.Status == BackupRunStatus.SucceededWithWarnings);
        Assert.Empty(logs);
    }

    [Fact]
    public async Task FinishRunAndMarkPolicyIdleAsync_ShouldNotOverwriteCancelledRun()
    {
        var setup = await CreateRepositoryPolicyAndRunAsync("backup-cancel-race", keepLastSuccessful: 1);
        BackupRunExecutionPlan plan;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            plan = await uow.BackupRuns.TryClaimExecutionPlanAsync(
                       setup.Run.Id,
                       DateTimeOffset.UtcNow,
                       TestContext.Current.CancellationToken)
                   ?? throw new InvalidOperationException("Backup run was not claimed.");
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var rows = await uow.BackupRuns.CancelQueuedOrRunningAsync(
                setup.Run.Id,
                DateTimeOffset.UtcNow,
                "Backup run cancelled.",
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
            Assert.Equal(1, rows);
        }

        plan.Run.MarkRunning(DateTimeOffset.UtcNow);
        plan.Run.CompleteSucceeded("snapshot-cancelled", null, 1, 1, 1, [], DateTimeOffset.UtcNow);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var result = await uow.BackupRuns.FinishRunAndMarkPolicyIdleAsync(
                plan.Run,
                plan.Policy.Id,
                successful: true,
                plan.Run.CompletedAt ?? DateTimeOffset.UtcNow,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
            Assert.Equal(BackupRunFinishResult.AlreadyCancelled, result);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var storedRun = await uow.BackupRuns.GetAsync(setup.Run.Id, TestContext.Current.CancellationToken);
            var storedPolicy = await uow.BackupPolicies.GetAsync(setup.Policy.Id, TestContext.Current.CancellationToken);

            Assert.NotNull(storedRun);
            Assert.Equal(BackupRunStatus.Cancelled, storedRun.Status);
            Assert.Null(storedRun.ResticSnapshotId);
            Assert.Equal(BackupSnapshotAvailability.NotCreated, storedRun.SnapshotAvailability);

            Assert.NotNull(storedPolicy);
            Assert.Equal(ResourceControlState.Idle, storedPolicy.ControlState);
            Assert.Null(storedPolicy.CurrentRunId);
            Assert.Null(storedPolicy.FirstSuccessfulRunAt);
        }
    }

    private async Task<BackupRunSetup> CreateRepositoryPolicyAndRunAsync(string name, int keepLastSuccessful)
    {
        var passwordSecretId = await CreateInternalSecretAsync($"{name.Replace('-', '_').ToUpperInvariant()}_PASSWORD", "restic-password");
        var repositoryPath = Path.Combine(testRoot, "data", "backups", "repositories", name);
        var repository = new BackupRepository(
            $"{name}-repository",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, Path.GetFullPath(repositoryPath)),
            passwordSecretId,
            Constants.SystemId);
        var policy = new BackupPolicy(
            $"{name}-policy",
            null,
            new CitadelSystemBackupSource(),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            webhook: null,
            keepLastSuccessful,
            timeoutSeconds: 120,
            alertOnFailure: false,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositories.AddAsync(repository, TestContext.Current.CancellationToken);
        await uow.BackupPolicies.AddAsync(policy, TestContext.Current.CancellationToken);
        var queue = await uow.BackupRuns.QueueAsync(
            policy.Id,
            Guid.CreateVersion7(),
            BackupRunTrigger.Manual,
            null,
            Constants.SystemId,
            usePolicyActor: false,
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        Assert.Equal(BackupRunQueueResultStatus.Queued, queue.Status);
        return new BackupRunSetup(repository, policy, queue.Run!);
    }

    private async Task<BackupRunSetup> CreateRemoteVolumeRepositoryPolicyAndRunAsync(Guid platformId, int keepLastSuccessful)
    {
        var passwordSecretId = await CreateInternalSecretAsync("REMOTE_VOLUME_RESTIC_PASSWORD", "restic-password");
        var accessKeySecretId = await CreateInternalSecretAsync("REMOTE_VOLUME_RESTIC_ACCESS_KEY", "access-key");
        var secretKeySecretId = await CreateInternalSecretAsync("REMOTE_VOLUME_RESTIC_SECRET_KEY", "secret-key");
        var repository = new BackupRepository(
            "remote-volume-repository",
            null,
            new S3CompatibleBackupRepositorySpec(
                new Uri("https://minio.example.com"),
                "citadel",
                "remote-volume",
                "us-east-1",
                S3BucketLookup.Path,
                accessKeySecretId,
                secretKeySecretId,
                SessionTokenSecretId: null),
            passwordSecretId,
            Constants.SystemId);
        var policy = new BackupPolicy(
            "remote-volume-policy",
            null,
            new DockerVolumeBackupSource(platformId, "remote-data"),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            webhook: null,
            keepLastSuccessful,
            timeoutSeconds: 120,
            alertOnFailure: false,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositories.AddAsync(repository, TestContext.Current.CancellationToken);
        await uow.BackupPolicies.AddAsync(policy, TestContext.Current.CancellationToken);
        var queue = await uow.BackupRuns.QueueAsync(
            policy.Id,
            Guid.CreateVersion7(),
            BackupRunTrigger.Manual,
            null,
            Constants.SystemId,
            usePolicyActor: false,
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        Assert.Equal(BackupRunQueueResultStatus.Queued, queue.Status);
        return new BackupRunSetup(repository, policy, queue.Run!);
    }

    private async Task<Guid> CreateInternalSecretAsync(string name, string value)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name, value },
            cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: TestContext.Current.CancellationToken);
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private sealed class FakeResticProcessRunner : IResticProcessRunner
    {
        private readonly Queue<ResticResponse> responses = new();

        public List<ResticProcessCall> Calls { get; } = [];

        public void Enqueue(int exitCode, string? stdout = null, string? stderr = null)
            => responses.Enqueue(new ResticResponse(exitCode, stdout, stderr));

        public async IAsyncEnumerable<ResticProcessEvent> RunAsync(
            ResticProcessCommand command,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            Calls.Add(new ResticProcessCall(command));
            var response = responses.Count > 0 ? responses.Dequeue() : new ResticResponse(0, "[]", null);
            await Task.Yield();

            if (response.Stdout is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdOut, response.Stdout.Trim());

            if (response.Stderr is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, response.Stderr);

            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: response.ExitCode);
        }
    }

    private sealed record BackupRunSetup(BackupRepository Repository, BackupPolicy Policy, BackupRun Run);
    private sealed record ResticResponse(int ExitCode, string? Stdout, string? Stderr);
    private sealed record ResticProcessCall(ResticProcessCommand Command);

    private sealed class FakeConnectorFactory<T>(T connector) : IConnectorFactory<T>
    {
        public T GetConnector(PlatformConnectorType type) => connector;
    }

    private sealed class FakeVolumeConnector : IVolumeConnector
    {
        public DockerVolumeResult? Volume { get; set; }

        public Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(ListdDockerVolumesCommand volumesCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success<IEnumerable<DockerVolumeResult>>(Volume is null ? [] : [Volume]));

        public Task<Result<DockerVolumeResult>> CreateVolumeAsync(CreateDockerVolumeCommand createVolumeCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(Volume!));

        public Task<Result<DockerVolumeResult>> InspectVolumeAsync(InspectDockerVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
            => Task.FromResult(Volume is null
                ? Result.Failure<DockerVolumeResult>(new Hosting.Common.ErrorTypes.NotFoundError("Volume not found."))
                : Result.Success(Volume));

        public Task<Result> DeleteVolumeAsync(DeleteDockerVolumeCommand removeVolumeCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success());
    }

    private sealed class FakeContainerConnector : IContainerConnector
    {
        private readonly Queue<ExecResponse> execResponses = new();

        public List<CreateContainerCommand> CreateCommands { get; } = [];
        public List<(string PlatformAddress, ContainerBinaryExecRequest Request)> ExecRequests { get; } = [];

        public void EnqueueExec(int exitCode, string? stdout = null, string? stderr = null)
            => execResponses.Enqueue(new ExecResponse(exitCode, stdout, stderr));

        public Task<Result<string>> CreateAsync(CreateContainerCommand createContainerCommand, CancellationToken cancellationToken)
        {
            CreateCommands.Add(createContainerCommand);
            return Task.FromResult(Result.Success("helper-01"));
        }

        public Task<Result> PatchAsync(PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success());

        public Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(new ContainerInspectionInfo(
                Id: inspectContainerCommand.ContainerId,
                Created: DateTimeOffset.UtcNow.ToString("O"),
                Path: null,
                Args: [],
                State: new ContainerRuntimeState(
                    ContainerStateStatus.Running,
                    Running: true,
                    Paused: false,
                    Restarting: false,
                    OOMKilled: false,
                    Dead: false,
                    Pid: 1,
                    ExitCode: null,
                    Error: null,
                    StartedAt: DateTimeOffset.UtcNow.ToString("O"),
                    FinishedAt: null,
                    Health: null),
                Image: null,
                ResolvConfPath: null,
                HostnamePath: null,
                HostsPath: null,
                LogPath: null,
                Name: "helper-01",
                RestartCount: 0,
                Driver: null,
                Platform: null,
                MountLabel: null,
                ProcessLabel: null,
                AppArmorProfile: null,
                ExecIDs: [],
                HostConfig: null,
                GraphDriver: null,
                SizeRw: null,
                SizeRootFs: null,
                Mounts: [],
                Config: null,
                NetworkSettings: null)));

        public Task<Result<ContainerBinaryExecResult>> ExecBinaryAsync(string platformAddress, ContainerBinaryExecRequest request, CancellationToken cancellationToken)
        {
            ExecRequests.Add((platformAddress, request));
            var response = execResponses.Count > 0 ? execResponses.Dequeue() : new ExecResponse(0, null, null);
            return Task.FromResult(Result.Success(new ContainerBinaryExecResult
            {
                Output = StreamExecAsync(response),
                GetExitCodeAsync = _ => Task.FromResult<int?>(response.ExitCode),
                CleanupAsync = () => ValueTask.CompletedTask
            }));
        }

        public Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success());

        public Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(ContainerFilterCommand containerFilterCommand, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(new Dictionary<string, DockerContainer>()));

        public Task<IExecSession> ExecAsync(string platformAddress, string containerId, string cmd, CancellationToken cancellationToken)
            => throw new NotSupportedException();

        public async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.CompletedTask;
            yield break;
        }

        public async IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.CompletedTask;
            yield break;
        }

        public async IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(StreamContainersStatsCommand streamStatsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.CompletedTask;
            yield break;
        }

        private static async IAsyncEnumerable<ContainerBinaryExecChunk> StreamExecAsync(ExecResponse response)
        {
            await Task.Yield();
            if (response.Stdout is not null)
                yield return new ContainerBinaryExecChunk(ContainerExecStream.Stdout, System.Text.Encoding.UTF8.GetBytes(response.Stdout));
            if (response.Stderr is not null)
                yield return new ContainerBinaryExecChunk(ContainerExecStream.Stderr, System.Text.Encoding.UTF8.GetBytes(response.Stderr));
        }
    }

    private sealed class FakeImageConnector : IImageConnector
    {
        public Task<Result<ImageResult>> GetAsync(string platformAddress, string imageId, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(string platformAddress, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<InspectImageResult>> InspectImageAsync(InspectImageCommand inspectImageCommand, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<DistributionResult>> DistributionInspectAsync(DistributionInspectCommand command, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<ExposedPortsResult>> GetExposedPortsAsync(RunImageInfoCommand runImageInfoCommand, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<IEnumerable<HistoryImageResult>>> HistoryImageAsync(HistoryImageCommand command, CancellationToken cancellationToken) => throw new NotSupportedException();
        public Task<Result<DeleteImageResult>> DeleteImageAsync(DeleteImageCommand deleteImageCommand, CancellationToken cancellationToken) => throw new NotSupportedException();

        public async IAsyncEnumerable<PullImageStreamItem> PullImageProgressStreamAsync(PullImageCommand pullImageCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await Task.CompletedTask;
            yield break;
        }
    }

    private sealed record ExecResponse(int ExitCode, string? Stdout, string? Stderr);
}
