use serde::{Deserialize, Serialize};

use super::{WebhookConfiguration, WebhookError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum WebhookProvider {
    #[default]
    GitHub,
    GitLab,
    Generic,
}

impl WebhookProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Generic => "Generic",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum WebhookAuthScheme {
    #[default]
    GitHubHmacSha256,
    GitLabSignedToken,
    GitLabLegacyToken,
    BearerToken,
}

impl WebhookAuthScheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitHubHmacSha256 => "GitHubHmacSha256",
            Self::GitLabSignedToken => "GitLabSignedToken",
            Self::GitLabLegacyToken => "GitLabLegacyToken",
            Self::BearerToken => "BearerToken",
        }
    }
}

/// Shared webhook wire configuration and defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RepoWebhookConfig {
    #[serde(alias = "Enabled")]
    pub enabled: bool,
    #[serde(alias = "Provider")]
    pub provider: WebhookProvider,
    #[serde(alias = "AuthScheme")]
    pub auth_scheme: WebhookAuthScheme,
    #[serde(alias = "Secret")]
    pub secret: Option<String>,
    #[serde(alias = "BranchFilter")]
    pub branch_filter: Option<String>,
}

impl RepoWebhookConfig {
    pub fn validate(&self) -> Result<(), WebhookError> {
        if self.secret.as_ref().is_some_and(|value| value.len() > 256)
            || self
                .branch_filter
                .as_ref()
                .is_some_and(|value| value.len() > 256)
        {
            return Err(WebhookError::Validation(
                "Webhook Secret and branch filter cannot exceed 256 characters.".into(),
            ));
        }
        let valid = !self.enabled
            || matches!(
                (self.provider, self.auth_scheme),
                (WebhookProvider::GitHub, WebhookAuthScheme::GitHubHmacSha256)
                    | (
                        WebhookProvider::GitLab,
                        WebhookAuthScheme::GitLabSignedToken | WebhookAuthScheme::GitLabLegacyToken
                    )
            )
            || (self.provider == WebhookProvider::Generic
                && self.auth_scheme == WebhookAuthScheme::BearerToken
                && self
                    .secret
                    .as_ref()
                    .is_some_and(|value| !value.trim().is_empty()));
        if !valid {
            return Err(WebhookError::Validation(
                "Webhook provider and authentication scheme are not compatible.".into(),
            ));
        }
        Ok(())
    }

    pub fn configuration(&self) -> Result<Option<WebhookConfiguration>, WebhookError> {
        self.validate()?;
        Ok(self.enabled.then(|| WebhookConfiguration {
            enabled: self.enabled,
            provider: self.provider.as_str().into(),
            auth_scheme: self.auth_scheme.as_str().into(),
            secret: self.secret.clone(),
            branch_filter: self.branch_filter.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_aliases_and_wire_names_match_the_existing_contract() {
        let default: RepoWebhookConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default, RepoWebhookConfig::default());
        assert!(default.configuration().unwrap().is_none());
        let config: RepoWebhookConfig = serde_json::from_value(json!({"Enabled":true,"Provider":"Generic","AuthScheme":"BearerToken","Secret":"token"})).unwrap();
        assert_eq!(config.configuration().unwrap().unwrap().provider, "Generic");
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["authScheme"], "BearerToken");
        assert_eq!(
            serde_json::from_value::<RepoWebhookConfig>(json).unwrap(),
            config
        );
    }

    #[test]
    fn typed_validation_matches_existing_webhook_rules() {
        for provider in [
            WebhookProvider::GitHub,
            WebhookProvider::GitLab,
            WebhookProvider::Generic,
        ] {
            for auth_scheme in [
                WebhookAuthScheme::GitHubHmacSha256,
                WebhookAuthScheme::GitLabSignedToken,
                WebhookAuthScheme::GitLabLegacyToken,
                WebhookAuthScheme::BearerToken,
            ] {
                for secret in [
                    None,
                    Some("".into()),
                    Some(" ".into()),
                    Some("token".into()),
                    Some("x".repeat(257)),
                ] {
                    for enabled in [false, true] {
                        let config = RepoWebhookConfig {
                            enabled,
                            provider,
                            auth_scheme,
                            secret: secret.clone(),
                            branch_filter: None,
                        };
                        let legacy = serde_json::to_value(&config).unwrap();
                        assert_eq!(
                            config.validate().is_ok(),
                            crate::repositories::webhooks::validate_webhook(Some(&legacy)).is_ok()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_types_enums_and_provider_scheme_combinations() {
        for json in [
            json!({"enabled":"true"}),
            json!({"provider":"Unknown"}),
            json!({"authScheme":"Unknown"}),
            json!({"secret":42}),
        ] {
            assert!(serde_json::from_value::<RepoWebhookConfig>(json).is_err());
        }
        for json in [
            json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken"}),
            json!({"enabled":true,"provider":"GitHub","authScheme":"BearerToken","secret":"token"}),
            json!({"secret":"x".repeat(257)}),
        ] {
            assert!(
                serde_json::from_value::<RepoWebhookConfig>(json)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
}
