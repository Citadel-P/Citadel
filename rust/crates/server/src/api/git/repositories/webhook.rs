use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum WebhookProvider {
    #[default]
    GitHub,
    GitLab,
    Generic,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum WebhookAuthScheme {
    #[default]
    GitHubHmacSha256,
    GitLabSignedToken,
    GitLabLegacyToken,
    BearerToken,
}
/// Shared wire configuration, matching .NET's WebhookConfig defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct RepoWebhookConfig {
    #[serde(alias = "Enabled")]
    pub enabled: bool,
    #[serde(alias = "Provider")]
    #[schema(inline)]
    pub provider: WebhookProvider,
    #[serde(alias = "AuthScheme")]
    #[schema(inline)]
    pub auth_scheme: WebhookAuthScheme,
    #[serde(alias = "Secret")]
    pub secret: Option<String>,
    #[serde(alias = "BranchFilter")]
    pub branch_filter: Option<String>,
}

impl From<WebhookProvider> for citadel_git::repositories::webhooks::WebhookProvider {
    fn from(value: WebhookProvider) -> Self {
        match value {
            WebhookProvider::GitHub => Self::GitHub,
            WebhookProvider::GitLab => Self::GitLab,
            WebhookProvider::Generic => Self::Generic,
        }
    }
}

impl From<citadel_git::repositories::webhooks::WebhookProvider> for WebhookProvider {
    fn from(value: citadel_git::repositories::webhooks::WebhookProvider) -> Self {
        match value {
            citadel_git::repositories::webhooks::WebhookProvider::GitHub => Self::GitHub,
            citadel_git::repositories::webhooks::WebhookProvider::GitLab => Self::GitLab,
            citadel_git::repositories::webhooks::WebhookProvider::Generic => Self::Generic,
        }
    }
}

impl From<WebhookAuthScheme> for citadel_git::repositories::webhooks::WebhookAuthScheme {
    fn from(value: WebhookAuthScheme) -> Self {
        match value {
            WebhookAuthScheme::GitHubHmacSha256 => Self::GitHubHmacSha256,
            WebhookAuthScheme::GitLabSignedToken => Self::GitLabSignedToken,
            WebhookAuthScheme::GitLabLegacyToken => Self::GitLabLegacyToken,
            WebhookAuthScheme::BearerToken => Self::BearerToken,
        }
    }
}

impl From<citadel_git::repositories::webhooks::WebhookAuthScheme> for WebhookAuthScheme {
    fn from(value: citadel_git::repositories::webhooks::WebhookAuthScheme) -> Self {
        match value {
            citadel_git::repositories::webhooks::WebhookAuthScheme::GitHubHmacSha256 => {
                Self::GitHubHmacSha256
            }
            citadel_git::repositories::webhooks::WebhookAuthScheme::GitLabSignedToken => {
                Self::GitLabSignedToken
            }
            citadel_git::repositories::webhooks::WebhookAuthScheme::GitLabLegacyToken => {
                Self::GitLabLegacyToken
            }
            citadel_git::repositories::webhooks::WebhookAuthScheme::BearerToken => {
                Self::BearerToken
            }
        }
    }
}

impl From<RepoWebhookConfig> for citadel_git::repositories::webhooks::RepoWebhookConfig {
    fn from(value: RepoWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider.into(),
            auth_scheme: value.auth_scheme.into(),
            secret: value.secret,
            branch_filter: value.branch_filter,
        }
    }
}

impl From<citadel_git::repositories::webhooks::RepoWebhookConfig> for RepoWebhookConfig {
    fn from(value: citadel_git::repositories::webhooks::RepoWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider.into(),
            auth_scheme: value.auth_scheme.into(),
            secret: value.secret,
            branch_filter: value.branch_filter,
        }
    }
}
