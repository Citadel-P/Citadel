using Application.Features.Backups.Commands;
using Application.Features.Backups.Models;
using Domain.Entities.Backups;

namespace Tests.Unit.Application.Features.Backups;

public sealed class BackupLicenseConfigurationPolicyTests
{
    [Fact]
    public void ChangesActivePaidTrigger_Should_Allow_Removing_A_Schedule_When_A_Webhook_Remains()
    {
        var policy = CreatePolicy(
            cron: "0 2 * * *",
            webhook: new BackupWebhookConfig(Enabled: true));
        var proposed = new UpdateBackupPolicyInputModel(Cron: null);

        var changed = BackupLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            policy,
            proposed,
            updateCron: true,
            updateTimeZone: false,
            updateWebhook: false);

        Assert.False(changed);
    }

    [Fact]
    public void ChangesActivePaidTrigger_Should_Detect_An_Active_Cron_Change()
    {
        var policy = CreatePolicy(cron: "0 2 * * *");
        var proposed = new UpdateBackupPolicyInputModel(Cron: "0 3 * * *");

        var changed = BackupLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            policy,
            proposed,
            updateCron: true,
            updateTimeZone: false,
            updateWebhook: false);

        Assert.True(changed);
    }

    [Fact]
    public void ChangesActivePaidTrigger_Should_Allow_Disabling_The_Policy()
    {
        var policy = CreatePolicy(cron: "0 2 * * *");
        var proposed = new UpdateBackupPolicyInputModel(Enabled: false);

        var changed = BackupLicenseConfigurationPolicy.ChangesActivePaidTrigger(
            policy,
            proposed,
            updateCron: false,
            updateTimeZone: false,
            updateWebhook: false);

        Assert.False(changed);
    }

    private static BackupPolicy CreatePolicy(
        string? cron,
        BackupWebhookConfig? webhook = null)
        => new(
            name: "daily-backup",
            description: null,
            source: new DockerVolumeBackupSource(Guid.CreateVersion7(), "data"),
            backupRepositoryId: Guid.CreateVersion7(),
            enabled: true,
            cron: cron,
            timeZone: "UTC",
            webhook: webhook,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Guid.CreateVersion7(),
            createdByActorId: Guid.CreateVersion7());
}
