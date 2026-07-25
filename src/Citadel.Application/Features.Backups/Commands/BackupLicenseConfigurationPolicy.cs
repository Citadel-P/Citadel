using Application.Features.Backups.Models;
using Domain.Entities.Backups;

namespace Application.Features.Backups.Commands;

internal static class BackupLicenseConfigurationPolicy
{
    public static bool ChangesActivePaidTrigger(
        BackupPolicy current,
        UpdateBackupPolicyInputModel proposed,
        bool updateCron,
        bool updateTimeZone,
        bool updateWebhook)
    {
        var resultingEnabled = proposed.Enabled ?? current.Enabled;
        if (!resultingEnabled)
            return false;

        var resultingCron = updateCron
            ? NormalizeOptional(proposed.Cron)
            : current.Cron;
        var resultingWebhook = updateWebhook
            ? NormalizeWebhook(proposed.Webhook)
            : current.Webhook;
        var hasResultingSchedule = resultingCron is not null;
        var hasResultingWebhook = resultingWebhook?.Enabled == true;

        var activatesPolicy = proposed.Enabled == true
            && !current.Enabled
            && (hasResultingSchedule || hasResultingWebhook);
        var changesActiveSchedule = hasResultingSchedule
            && ((updateCron
                    && !string.Equals(
                        resultingCron,
                        current.Cron,
                        StringComparison.Ordinal))
                || (updateTimeZone
                    && !string.Equals(
                        NormalizeOptional(proposed.TimeZone),
                        current.TimeZone,
                        StringComparison.Ordinal)));
        var changesActiveWebhook = hasResultingWebhook
            && updateWebhook
            && (!current.WebhookEnabled || resultingWebhook != current.Webhook);

        return activatesPolicy || changesActiveSchedule || changesActiveWebhook;
    }

    private static string? NormalizeOptional(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    private static BackupWebhookConfig? NormalizeWebhook(BackupWebhookConfig? value)
        => value is null
            ? null
            : value with
            {
                Secret = NormalizeOptional(value.Secret),
                BranchFilter = NormalizeOptional(value.BranchFilter)
            };
}
