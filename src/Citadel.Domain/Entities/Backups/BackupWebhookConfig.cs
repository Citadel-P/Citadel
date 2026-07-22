using Domain.Contracts.Resources;

namespace Domain.Entities.Backups;

public sealed record BackupWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);

public sealed record BackupRunWarning(string Code, string Message);

public sealed record BackupAffectedContainer(
    string DockerContainerId,
    string Name,
    ContainerStateStatus OriginalState,
    bool StopAttempted,
    bool RestartAttempted,
    bool RestartSucceeded);
