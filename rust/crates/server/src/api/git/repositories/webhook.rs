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
