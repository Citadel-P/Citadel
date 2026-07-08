using Domain;
using Domain.Entities.Automation;

namespace Tests.Unit.Domain.Entities.Automation;

public sealed class AutomationActionTests
{
    [Fact]
    public void Validate_ShouldRequireScheduleCronWhenScheduleIsEnabled()
    {
        var action = CreateAction(scheduleEnabled: true, scheduleCron: null);

        var ex = Assert.Throws<ArgumentException>(action.Validate);

        Assert.Equal("Schedule cron is required when the schedule is enabled. (Parameter 'ScheduleCron')", ex.Message);
    }

    [Fact]
    public void Update_ShouldNormalizeDescriptionDefaultArgsAndWebhookFields()
    {
        var action = CreateAction();

        action.Update(
            description: "  updated description  ",
            updateDescription: true,
            code: "console.log('updated');",
            defaultArgsJson: """  {"enabled":true}  """,
            enabled: false,
            scheduleEnabled: true,
            scheduleCron: "  0 * * * *  ",
            updateScheduleCron: true,
            scheduleTimeZone: "  Europe/Paris  ",
            webhook: new AutomationWebhookConfig(
                Enabled: true,
                Secret: "  secret  ",
                BranchFilter: "  main  "),
            updateWebhook: true,
            timeoutSeconds: 120,
            alertOnFailure: true,
            runAsActorId: Guid.NewGuid());

        Assert.Equal("updated description", action.Description);
        Assert.Equal("console.log('updated');", action.Code);
        Assert.Equal("""{"enabled":true}""", action.DefaultArgsJson);
        Assert.False(action.Enabled);
        Assert.True(action.ScheduleEnabled);
        Assert.Equal("0 * * * *", action.ScheduleCron);
        Assert.Equal("Europe/Paris", action.ScheduleTimeZone);
        Assert.True(action.WebhookEnabled);
        Assert.Equal("secret", action.Webhook?.Secret);
        Assert.Equal("main", action.Webhook?.BranchFilter);
        Assert.Equal(120, action.TimeoutSeconds);
        Assert.True(action.AlertOnFailure);
        Assert.True(action.RowVersion > 0);
    }

    [Fact]
    public void ActionRun_ShouldRecordCompletionDetails()
    {
        var run = new ActionRun(
            actionId: Guid.NewGuid(),
            actionName: "action-1",
            trigger: ActionRunTrigger.Test,
            runAsActorId: Guid.NewGuid(),
            triggeredByActorId: Guid.NewGuid(),
            argsJson: "  ",
            codeSnapshot: "console.log('test');",
            timeoutSeconds: 30);
        var startedAt = DateTime.UtcNow;

        run.MarkRunning(startedAt);
        run.Complete(ActionRunStatus.Failed, 1, "log output", "  failed  ", startedAt.AddMilliseconds(42));

        Assert.Equal("{}", run.ArgsJson);
        Assert.Equal(ActionRunStatus.Failed, run.Status);
        Assert.Equal(1, run.ExitCode);
        Assert.Equal("log output", run.Logs);
        Assert.Equal("failed", run.ErrorMessage);
        Assert.Equal(42, run.DurationMs);
        Assert.NotEmpty(run.CodeHash);
    }

    private static AutomationAction CreateAction(bool scheduleEnabled = false, string? scheduleCron = null)
        => new(
            name: " action-1 ",
            description: null,
            code: "console.log('hello');",
            defaultArgsJson: "  ",
            enabled: true,
            scheduleEnabled: scheduleEnabled,
            scheduleCron: scheduleCron,
            scheduleTimeZone: "UTC",
            webhook: null,
            timeoutSeconds: 30,
            alertOnFailure: false,
            runAsActorId: Guid.NewGuid(),
            createdByActorId: Guid.NewGuid());
}
