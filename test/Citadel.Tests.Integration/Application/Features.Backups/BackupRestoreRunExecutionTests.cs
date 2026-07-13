using Application.Configs;
using Application.Services;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Backups;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using System.Collections.Immutable;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Backups;

public sealed class BackupRestoreRunExecutionTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeResticProcessRunner restic = new();
    private readonly FakeVolumeConnector volumeConnector = new();
    private readonly string testRoot = Path.Combine(Path.GetTempPath(), $"citadel-backup-restore-test-{Guid.NewGuid():N}");

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<IResticProcessRunner>(restic);
        services.ReplaceService<IConnectorFactory<IVolumeConnector>>(new FakeVolumeConnectorFactory(volumeConnector));
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
    public async Task ExecuteQueuedAsync_ShouldRestoreVolumeAndPersistLogs()
    {
        var platformId = await SeedLocalPlatformAsync();
        var sourceMountpoint = CreateVolumeMountpoint("source-volume");
        volumeConnector.AddVolume("source-volume", sourceMountpoint);
        restic.Enqueue(exitCode: 0, stdout: "restored files");
        var setup = await CreateRepositoryBackupRunAndRestoreRunAsync(
            "restore-success",
            platformId,
            sourceVolumeName: "source-volume",
            targetVolumeName: "target-volume",
            overwriteExisting: false);

        var service = Services.GetRequiredService<IBackupRestoreRunExecutionService>();
        var items = new List<BackupRestoreRunStreamItem>();
        await foreach (var item in service.ExecuteQueuedAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.Status == BackupRestoreStatus.Running);
        Assert.Contains(items, item => item.Status == BackupRestoreStatus.Succeeded);

        Assert.Single(restic.Calls);
        var args = restic.Calls[0].Command.Arguments;
        Assert.Contains("restore", args);
        Assert.Contains($"snapshot-restore-001:{Path.GetFullPath(sourceMountpoint)}", args);
        Assert.Contains("--target", args);
        Assert.Contains(Path.GetFullPath(volumeConnector.GetVolume("target-volume").Mountpoint), args);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRestoreRuns.GetAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken);
        var logs = await uow.BackupRestoreRunLogs.GetByRunAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRestoreStatus.Succeeded, storedRun.Status);
        Assert.True(storedRun.TargetVolumeCreatedByCitadel);
        Assert.Null(storedRun.ErrorCode);
        Assert.Contains(logs, log => log.Stream == "stdout" && log.Message == "restored files");

        var repositoryLeaseAvailable = await uow.BackupRepositoryLeases.TryAcquireAsync(
            setup.Repository.Id,
            "Test",
            Guid.CreateVersion7(),
            DateTimeOffset.UtcNow.AddMinutes(1),
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        var sourceLeaseAvailable = await uow.BackupSourceLeases.TryAcquireAsync(
            $"{platformId}:target-volume",
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
    public async Task ExecuteQueuedAsync_ShouldFailRunAndPersistStderr_WhenResticFails()
    {
        var platformId = await SeedLocalPlatformAsync();
        volumeConnector.AddVolume("source-volume", CreateVolumeMountpoint("source-volume"));
        restic.Enqueue(exitCode: 1, stderr: "restore failed");
        var setup = await CreateRepositoryBackupRunAndRestoreRunAsync(
            "restore-failure",
            platformId,
            sourceVolumeName: "source-volume",
            targetVolumeName: "target-volume",
            overwriteExisting: false);

        var service = Services.GetRequiredService<IBackupRestoreRunExecutionService>();
        await foreach (var _ in service.ExecuteQueuedAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken))
        {
        }

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRestoreRuns.GetAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken);
        var logs = await uow.BackupRestoreRunLogs.GetByRunAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRun);
        Assert.Equal(BackupRestoreStatus.Failed, storedRun.Status);
        Assert.Equal(1, storedRun.ExitCode);
        Assert.Contains("Restic restore exited with code 1", storedRun.ErrorMessage);
        Assert.Contains(logs, log => log.Stream == "stderr" && log.Message == "restore failed");
    }

    [Fact]
    public async Task ExecuteQueuedAsync_ShouldReject_WhenTargetVolumeExistsAndOverwriteIsDisabled()
    {
        var platformId = await SeedLocalPlatformAsync();
        volumeConnector.AddVolume("source-volume", CreateVolumeMountpoint("source-volume"));
        volumeConnector.AddVolume("target-volume", CreateVolumeMountpoint("target-volume"));
        var setup = await CreateRepositoryBackupRunAndRestoreRunAsync(
            "restore-target-conflict",
            platformId,
            sourceVolumeName: "source-volume",
            targetVolumeName: "target-volume",
            overwriteExisting: false);

        var service = Services.GetRequiredService<IBackupRestoreRunExecutionService>();
        await foreach (var _ in service.ExecuteQueuedAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken))
        {
        }

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRun = await uow.BackupRestoreRuns.GetAsync(setup.RestoreRun.Id, TestContext.Current.CancellationToken);

        Assert.Empty(restic.Calls);
        Assert.NotNull(storedRun);
        Assert.Equal(BackupRestoreStatus.Rejected, storedRun.Status);
        Assert.Equal("backup.restore.target_unavailable", storedRun.ErrorCode);
        Assert.Contains("Target volume already exists", storedRun.ErrorMessage);
    }

    private async Task<Guid> SeedLocalPlatformAsync()
    {
        var platform = new Platform(
            name: $"restore-platform-{Guid.CreateVersion7():N}",
            address: Constants.LocalDockerHostUrl,
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: "test",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Local,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "restore-platform",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0));

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var cache = Services.GetRequiredService<IPlatformContainerCache>();
        cache.ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(platform.Id, Constants.LocalDockerHostUrl, PlatformConnectorType.Local, ImmutableDictionary<string, Guid>.Empty));

        return platform.Id;
    }

    private string CreateVolumeMountpoint(string volumeName)
    {
        var path = Path.Combine(testRoot, "volumes", volumeName, "_data");
        Directory.CreateDirectory(path);
        return path;
    }

    private async Task<BackupRestoreRunSetup> CreateRepositoryBackupRunAndRestoreRunAsync(
        string name,
        Guid platformId,
        string sourceVolumeName,
        string targetVolumeName,
        bool overwriteExisting)
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
            new DockerVolumeBackupSource(platformId, sourceVolumeName),
            repository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            keepLastSuccessful: 2,
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

        Assert.Equal(BackupRunQueueResultStatus.Queued, queue.Status);
        var backupRun = queue.Run!;
        backupRun.MarkRunning(DateTimeOffset.UtcNow);
        backupRun.CompleteSucceeded("snapshot-restore-001", null, 1, 1, 1, [], DateTimeOffset.UtcNow);
        await uow.BackupRuns.FinishRunAndMarkPolicyIdleAsync(
            backupRun,
            policy.Id,
            successful: true,
            backupRun.CompletedAt ?? DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);

        var restoreRun = new BackupRestoreRun(
            backupRun.Id,
            repository.Id,
            platformId,
            targetVolumeName,
            overwriteExisting,
            Constants.SystemId);
        await uow.BackupRestoreRuns.AddAsync(restoreRun, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return new BackupRestoreRunSetup(repository, backupRun, restoreRun);
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
            var response = responses.Count > 0 ? responses.Dequeue() : new ResticResponse(0, null, null);
            await Task.Yield();

            if (response.Stdout is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdOut, response.Stdout.Trim());

            if (response.Stderr is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, response.Stderr);

            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: response.ExitCode);
        }
    }

    private sealed class FakeVolumeConnectorFactory(IVolumeConnector connector) : IConnectorFactory<IVolumeConnector>
    {
        public IVolumeConnector GetConnector(PlatformConnectorType type) => connector;
    }

    private sealed class FakeVolumeConnector : IVolumeConnector
    {
        private readonly Dictionary<string, DockerVolumeResult> volumes = new(StringComparer.Ordinal);

        public void AddVolume(string name, string mountpoint, bool inUse = false, IReadOnlyList<ContainerVolumeResult>? containers = null)
            => volumes[name] = CreateResult(name, mountpoint, inUse, containers ?? []);

        public DockerVolumeResult GetVolume(string name) => volumes[name];

        public Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(ListdDockerVolumesCommand volumesCommand, CancellationToken cancellationToken)
        {
            IEnumerable<DockerVolumeResult> result = volumes.Values;
            if (!string.IsNullOrWhiteSpace(volumesCommand.Name))
                result = result.Where(volume => string.Equals(volume.Name, volumesCommand.Name, StringComparison.Ordinal));

            return Task.FromResult(Result.Success(result));
        }

        public Task<Result<DockerVolumeResult>> CreateVolumeAsync(CreateDockerVolumeCommand createVolumeCommand, CancellationToken cancellationToken)
        {
            var mountpoint = Path.Combine(Path.GetTempPath(), "citadel-backup-restore-test-created", createVolumeCommand.Name, "_data");
            Directory.CreateDirectory(mountpoint);
            var volume = CreateResult(createVolumeCommand.Name, mountpoint, inUse: false, []);
            volumes[createVolumeCommand.Name] = volume;
            return Task.FromResult(Result.Success(volume));
        }

        public Task<Result<DockerVolumeResult>> InspectVolumeAsync(InspectDockerVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
            => Task.FromResult(volumes.TryGetValue(inspectVolumeCommand.Name, out var volume)
                ? Result.Success(volume)
                : Result.Failure<DockerVolumeResult>(new NotFoundError("Volume not found.")));

        public Task<Result> DeleteVolumeAsync(DeleteDockerVolumeCommand removeVolumeCommand, CancellationToken cancellationToken)
        {
            foreach (var name in removeVolumeCommand.Names)
            {
                if (!volumes.Remove(name))
                    return Task.FromResult(Result.Failure(new NotFoundError("Volume not found.")));
            }

            return Task.FromResult(Result.Success());
        }

        private static DockerVolumeResult CreateResult(
            string name,
            string mountpoint,
            bool inUse,
            IReadOnlyList<ContainerVolumeResult> containers)
            => new(
                Id: Guid.CreateVersion7().ToString("N"),
                Name: name,
                InUse: inUse,
                Scope: "local",
                Driver: "local",
                Mountpoint: mountpoint,
                CreatedAt: DateTimeOffset.UtcNow.ToString("O"),
                ClusterVolume: null,
                UsageData: null,
                Containers: containers,
                Status: new Dictionary<string, string>(),
                Labels: new Dictionary<string, string>(),
                Options: new Dictionary<string, string>());
    }

    private sealed record BackupRestoreRunSetup(BackupRepository Repository, BackupRun BackupRun, BackupRestoreRun RestoreRun);
    private sealed record ResticResponse(int ExitCode, string? Stdout, string? Stderr);
    private sealed record ResticProcessCall(ResticProcessCommand Command);
}
