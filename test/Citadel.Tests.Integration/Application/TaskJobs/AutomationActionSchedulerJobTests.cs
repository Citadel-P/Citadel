using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;

namespace Tests.Integration.Application.TaskJobs;

public sealed class AutomationActionSchedulerJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FixedTimeProvider timeProvider =
        new(new DateTimeOffset(2026, 7, 14, 8, 30, 45, TimeSpan.Zero));

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<TimeProvider>();
        services.AddSingleton<TimeProvider>(timeProvider);
    }

    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldNotDuplicateRunAcrossSchedulerRestart()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actionId = await CreateScheduledActionAsync(
            "restart-scheduled-action",
            cron: "* * * * *",
            timeZone: "UTC",
            cancellationToken: cancellationToken);

        var beforeRestart = ActivatorUtilities.CreateInstance<AutomationActionScheduler>(Services);
        await beforeRestart.QueueDueScheduledRunsAsync(cancellationToken);

        var afterRestart = ActivatorUtilities.CreateInstance<AutomationActionScheduler>(Services);
        await afterRestart.QueueDueScheduledRunsAsync(cancellationToken);

        var (action, runs) = await GetActionAndRunsAsync(actionId, cancellationToken);

        Assert.Equal(new DateTime(2026, 7, 14, 8, 30, 0, DateTimeKind.Utc), action.LastScheduledRunAt);
        var run = Assert.Single(runs);
        Assert.Equal(ActionRunTrigger.Schedule, run.Trigger);
    }

    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldClaimScheduledMinuteOnceAcrossConcurrentTicks()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actionId = await CreateScheduledActionAsync(
            "concurrent-scheduled-action",
            cron: "* * * * *",
            timeZone: "UTC",
            cancellationToken: cancellationToken);
        var firstScheduler = ActivatorUtilities.CreateInstance<AutomationActionScheduler>(Services);
        var secondScheduler = ActivatorUtilities.CreateInstance<AutomationActionScheduler>(Services);

        await Task.WhenAll(
            firstScheduler.QueueDueScheduledRunsAsync(cancellationToken),
            secondScheduler.QueueDueScheduledRunsAsync(cancellationToken));

        var (_, runs) = await GetActionAndRunsAsync(actionId, cancellationToken);

        Assert.Single(runs);
    }

    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldEvaluateCronInConfiguredTimeZone()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actionId = await CreateScheduledActionAsync(
            "timezone-scheduled-action",
            cron: "30 10 * * *",
            timeZone: "Europe/Paris",
            cancellationToken: cancellationToken);
        var scheduler = Services.GetRequiredService<IAutomationActionScheduler>();

        await scheduler.QueueDueScheduledRunsAsync(cancellationToken);

        var (_, runs) = await GetActionAndRunsAsync(actionId, cancellationToken);

        Assert.Single(runs);
    }

    [Fact]
    public async Task QueueDueScheduledRunsAsync_ShouldNotConsumeMinuteWhenQueueValidationFails()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actionId = await CreateScheduledActionAsync(
            "invalid-scheduled-action",
            cron: "* * * * *",
            timeZone: "UTC",
            timeoutSeconds: 1_801,
            cancellationToken: cancellationToken);
        var scheduler = Services.GetRequiredService<IAutomationActionScheduler>();

        await scheduler.QueueDueScheduledRunsAsync(cancellationToken);

        var (action, runs) = await GetActionAndRunsAsync(actionId, cancellationToken);

        Assert.Null(action.LastScheduledRunAt);
        Assert.Empty(runs);
    }

    private async Task<Guid> CreateScheduledActionAsync(
        string name,
        string cron,
        string timeZone,
        CancellationToken cancellationToken,
        int timeoutSeconds = 60)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = new AutomationAction(
            name,
            description: null,
            code: "console.log('scheduled');",
            defaultArgsJson: "{}",
            enabled: true,
            scheduleEnabled: true,
            scheduleCron: cron,
            scheduleTimeZone: timeZone,
            webhook: null,
            timeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);
        await unitOfWork.AutomationActions.AddAsync(action, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return action.Id;
    }

    private async Task<(AutomationAction Action, ActionRun[] Runs)> GetActionAndRunsAsync(
        Guid actionId,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await unitOfWork.AutomationActions.GetAsync(actionId, cancellationToken);
        var runs = (await unitOfWork.ActionRuns.GetByActionAsync(actionId, 50, cancellationToken)).ToArray();

        return (Assert.IsType<AutomationAction>(action), runs);
    }

    private sealed class FixedTimeProvider(DateTimeOffset now) : TimeProvider
    {
        public override DateTimeOffset GetUtcNow() => now;
    }
}
