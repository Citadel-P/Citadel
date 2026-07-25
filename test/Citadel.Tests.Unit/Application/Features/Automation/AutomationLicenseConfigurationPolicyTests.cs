using Application.Features.Automation.Commands;
using Application.Features.Automation.Models;
using Domain.Entities.Automation;

namespace Tests.Unit.Application.Features.Automation;

public sealed class AutomationLicenseConfigurationPolicyTests
{
    [Fact]
    public void ChangesActivePaidTrigger_Should_Ignore_Unchanged_Fields_Sent_By_The_Form()
    {
        var action = CreateScheduledAction();
        var proposed = Update(
            code: "console.log('updated');",
            enabled: true,
            scheduleEnabled: true,
            scheduleTimeZone: "UTC");

        var changed = AutomationLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            action,
            proposed,
            updateScheduleCron: false,
            updateWebhook: false);

        Assert.False(changed);
    }

    [Fact]
    public void ChangesActivePaidTrigger_Should_Detect_A_Cron_Change()
    {
        var action = CreateScheduledAction();
        var proposed = Update(scheduleCron: "*/10 * * * *");

        var changed = AutomationLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            action,
            proposed,
            updateScheduleCron: true,
            updateWebhook: false);

        Assert.True(changed);
    }

    [Fact]
    public void ChangesActivePaidTrigger_Should_Allow_Disabling_The_Last_Paid_Trigger()
    {
        var action = CreateScheduledAction();
        var proposed = Update(scheduleEnabled: false);

        var changed = AutomationLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            action,
            proposed,
            updateScheduleCron: false,
            updateWebhook: false);

        Assert.False(changed);
    }

    [Fact]
    public void ChangesActivePaidTrigger_Should_Allow_Disabling_A_Schedule_When_A_Webhook_Remains()
    {
        var action = CreateScheduledAction(
            new AutomationWebhookConfig(Enabled: true));
        var proposed = Update(scheduleEnabled: false);

        var changed = AutomationLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            action,
            proposed,
            updateScheduleCron: false,
            updateWebhook: false);

        Assert.False(changed);
    }

    private static AutomationAction CreateScheduledAction(
        AutomationWebhookConfig? webhook = null)
        => new(
            name: "scheduled-action",
            description: null,
            code: "console.log('original');",
            defaultArgsJson: "{}",
            enabled: true,
            scheduleEnabled: true,
            scheduleCron: "*/5 * * * *",
            scheduleTimeZone: "UTC",
            webhook: webhook,
            timeoutSeconds: 300,
            alertOnFailure: true,
            runAsActorId: Guid.CreateVersion7(),
            createdByActorId: Guid.CreateVersion7());

    private static UpdateAutomationActionInputModel Update(
        string? code = null,
        bool? enabled = null,
        bool? scheduleEnabled = null,
        string? scheduleCron = null,
        string? scheduleTimeZone = null)
        => new(
            Description: null,
            Code: code,
            DefaultArgsJson: null,
            Enabled: enabled,
            ScheduleEnabled: scheduleEnabled,
            ScheduleCron: scheduleCron,
            ScheduleTimeZone: scheduleTimeZone,
            Webhook: null,
            TimeoutSeconds: null,
            AlertOnFailure: null,
            RunAsActorId: null);
}
