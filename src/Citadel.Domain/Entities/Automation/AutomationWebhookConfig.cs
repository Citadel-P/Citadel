using Domain.Contracts.Resources;

namespace Domain.Entities.Automation;

public sealed record AutomationWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);
