//! Compatibility exports; webhook evaluation belongs to Git.
pub use citadel_git::repositories::webhooks::{
    WebhookConfiguration, WebhookError, repository_matches, webhook_branch,
};

mod config;
pub use config::{RepoWebhookConfig, WebhookAuthScheme, WebhookProvider};
