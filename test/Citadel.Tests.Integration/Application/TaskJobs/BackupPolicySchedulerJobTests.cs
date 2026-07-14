using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.TaskJobs;

public sealed class BackupPolicySchedulerJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldQueueDuePolicyOnce()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var policyId = await CreateScheduledPolicyAsync("scheduled-backup", enabled: true, cron: "* * * * *", cancellationToken);
        var scheduler = Services.GetRequiredService<IBackupPolicyScheduler>();

        await scheduler.QueueDueScheduledRunsAsync(cancellationToken);
        await scheduler.QueueDueScheduledRunsAsync(cancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var policy = await uow.BackupPolicies.GetAsync(policyId, cancellationToken);
        var runs = (await uow.BackupRuns.GetByPolicyAsync(policyId, 50, cancellationToken)).ToArray();

        Assert.NotNull(policy);
        Assert.NotNull(policy.LastScheduledRunAt);
        var run = Assert.Single(runs);
        Assert.Equal(BackupRunTrigger.Schedule, run.Trigger);
        Assert.Equal(BackupRunStatus.Queued, run.Status);
        Assert.Equal(Constants.SystemId, run.TriggeredByActorId);
    }

    [Fact]
    public async Task QueueScheduledAsync_ShouldMarkPolicyAndQueueRunInOneCommand()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var policyId = await CreateScheduledPolicyAsync("scheduled-single-command-backup", enabled: true, cron: "* * * * *", cancellationToken);
        var scheduledMinute = new DateTimeOffset(2026, 7, 14, 12, 30, 0, TimeSpan.Zero);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var first = await uow.BackupRuns.QueueScheduledAsync(
            policyId,
            Guid.CreateVersion7(),
            scheduledMinute,
            cancellationToken);
        var duplicate = await uow.BackupRuns.QueueScheduledAsync(
            policyId,
            Guid.CreateVersion7(),
            scheduledMinute,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var policy = await uow.BackupPolicies.GetAsync(policyId, cancellationToken);
        var runs = (await uow.BackupRuns.GetByPolicyAsync(policyId, 50, cancellationToken)).ToArray();

        Assert.Equal(BackupRunQueueResultStatus.Queued, first.Status);
        Assert.Equal(BackupRunQueueResultStatus.AlreadyScheduled, duplicate.Status);
        Assert.NotNull(first.Run);
        Assert.Equal(BackupRunTrigger.Schedule, first.Run.Trigger);
        Assert.Equal(Constants.SystemId, first.Run.TriggeredByActorId);
        Assert.NotNull(policy);
        Assert.Equal(scheduledMinute.UtcDateTime, policy.LastScheduledRunAt?.UtcDateTime);
        Assert.Single(runs);
    }

    [Fact]
    public async Task TryMarkScheduledAsync_ShouldPreventDuplicateScheduledMinute()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var policyId = await CreateScheduledPolicyAsync("scheduled-mark-backup", enabled: true, cron: "* * * * *", cancellationToken);
        var scheduledMinute = new DateTimeOffset(2026, 7, 14, 10, 15, 0, TimeSpan.Zero);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var first = await uow.BackupPolicies.TryMarkScheduledAsync(policyId, scheduledMinute, cancellationToken);
        var duplicate = await uow.BackupPolicies.TryMarkScheduledAsync(policyId, scheduledMinute, cancellationToken);
        var nextMinute = await uow.BackupPolicies.TryMarkScheduledAsync(policyId, scheduledMinute.AddMinutes(1), cancellationToken);
        await uow.CommitAsync(cancellationToken);

        Assert.True(first);
        Assert.False(duplicate);
        Assert.True(nextMinute);
    }

    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldIgnoreDisabledPolicies()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var policyId = await CreateScheduledPolicyAsync("disabled-scheduled-backup", enabled: false, cron: "* * * * *", cancellationToken);
        var scheduler = Services.GetRequiredService<IBackupPolicyScheduler>();

        await scheduler.QueueDueScheduledRunsAsync(cancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var policy = await uow.BackupPolicies.GetAsync(policyId, cancellationToken);
        var runs = (await uow.BackupRuns.GetByPolicyAsync(policyId, 50, cancellationToken)).ToArray();

        Assert.NotNull(policy);
        Assert.Null(policy.LastScheduledRunAt);
        Assert.Empty(runs);
    }

    private async Task<Guid> CreateScheduledPolicyAsync(
        string name,
        bool enabled,
        string cron,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var passwordSecret = new SecretDefinition($"{name.Replace('-', '_').ToUpperInvariant()}_RESTIC_PASSWORD", SecretProviderType.InternalEncrypted);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted-value"),
            cancellationToken);

        var repository = new BackupRepository(
            $"{name}-repository",
            null,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, $"/backup/{name}"),
            passwordSecret.Id,
            Constants.SystemId);
        await uow.BackupRepositories.AddAsync(repository, cancellationToken);

        var policy = new BackupPolicy(
            $"{name}-policy",
            null,
            new CitadelSystemBackupSource(),
            repository.Id,
            enabled,
            cron,
            timeZone: "UTC",
            webhook: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);
        await uow.BackupPolicies.AddAsync(policy, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        return policy.Id;
    }
}
