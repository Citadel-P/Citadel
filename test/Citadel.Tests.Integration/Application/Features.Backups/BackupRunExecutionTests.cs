using Application.Configs;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Backups;

public sealed class BackupRunExecutionTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeResticProcessRunner restic = new();
    private readonly string testRoot = Path.Combine(Path.GetTempPath(), $"citadel-backup-runs-test-{Guid.NewGuid():N}");

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<IResticProcessRunner>(restic);
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
}
