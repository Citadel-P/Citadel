using Application.Features.Automation.Models;
using Domain.Entities.Automation;

namespace Application.Features.Automation.Commands;

internal static class AutomationLicenseConfigurationPolicy
{
    public static bool ChangesActivePaidTrigger(
        AutomationAction current,
        UpdateAutomationActionInputModel proposed,
        bool updateScheduleCron,
        bool updateWebhook)
    {
        var resultingEnabled = proposed.Enabled ?? current.Enabled;
        var resultingScheduleEnabled = proposed.ScheduleEnabled ?? current.ScheduleEnabled;
        var resultingWebhook = updateWebhook
            ? NormalizeWebhook(proposed.Webhook)
            : current.Webhook;
        var resultingWebhookEnabled = resultingWebhook?.Enabled == true;

        if (!resultingEnabled)
            return false;

        var activatesAction = proposed.Enabled == true
            && !current.Enabled
            && (resultingScheduleEnabled || resultingWebhookEnabled);
        var changesActiveSchedule = resultingScheduleEnabled
            && ((!current.ScheduleEnabled && proposed.ScheduleEnabled == true)
                || (updateScheduleCron
                    && !string.Equals(
                        NormalizeOptional(proposed.ScheduleCron),
                        current.ScheduleCron,
                        StringComparison.Ordinal))
                || (proposed.ScheduleTimeZone is not null
                    && !string.Equals(
                        proposed.ScheduleTimeZone.Trim(),
                        current.ScheduleTimeZone,
                        StringComparison.Ordinal)));
        var changesActiveWebhook = resultingWebhookEnabled
            && updateWebhook
            && (!current.WebhookEnabled || resultingWebhook != current.Webhook);

        return activatesAction || changesActiveSchedule || changesActiveWebhook;
    }

    private static string? NormalizeOptional(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    private static AutomationWebhookConfig? NormalizeWebhook(AutomationWebhookConfig? value)
        => value is null
            ? null
            : value with
            {
                Secret = NormalizeOptional(value.Secret),
                BranchFilter = NormalizeOptional(value.BranchFilter)
            };
}
