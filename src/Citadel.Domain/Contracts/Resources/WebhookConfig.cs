namespace Domain.Contracts.Resources;

public abstract record WebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null);

public static class WebhookConfigurationValidation
{
    public static string? GetAuthenticationError(WebhookConfig? webhook) => webhook is null || !webhook.Enabled
        ? null
        : GetAuthenticationError(webhook.Provider, webhook.AuthScheme, webhook.Secret);

    public static string? GetAuthenticationError(
        WebhookProvider provider,
        WebhookAuthScheme authScheme,
        string? secret)
    {
        if (!Enum.IsDefined(provider))
            return "The webhook provider is not supported.";
        if (!Enum.IsDefined(authScheme))
            return "The webhook authentication scheme is not supported.";

        return provider switch
        {
            WebhookProvider.GitHub when authScheme != WebhookAuthScheme.GitHubHmacSha256
                => "GitHub webhooks require GitHub HMAC SHA-256 authentication.",
            WebhookProvider.GitLab when authScheme is not (WebhookAuthScheme.GitLabSignedToken or WebhookAuthScheme.GitLabLegacyToken)
                => "GitLab webhooks require a supported GitLab authentication scheme.",
            WebhookProvider.Generic when authScheme != WebhookAuthScheme.BearerToken
                => "Generic webhooks require shared-secret authentication using the Authorization: Bearer header.",
            WebhookProvider.Generic when string.IsNullOrWhiteSpace(secret)
                => "Generic webhooks require a shared secret.",
            _ => null
        };
    }
}
