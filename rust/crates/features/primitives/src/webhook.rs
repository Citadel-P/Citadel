use serde::{Deserialize, Serialize};

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
pub struct WebhookConfig {
    pub enabled: bool,
    pub provider: WebhookProvider,
    pub auth_scheme: WebhookAuthScheme,
    pub secret: Option<String>,
    pub branch_filter: Option<String>,
}

impl WebhookConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.secret.as_ref().is_some_and(|value| value.len() > 256)
            || self
                .branch_filter
                .as_ref()
                .is_some_and(|value| value.len() > 256)
        {
            return Err("Webhook Secret and branch filter cannot exceed 256 characters.");
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
            return Err("Webhook provider and authentication scheme are not compatible.");
        }
        Ok(())
    }

    /// Audit snapshots must never persist the credential itself.
    pub fn redacted(&self) -> Self {
        let mut value = self.clone();
        value.secret = value.secret.as_ref().map(|_| "********".into());
        value
    }
}

/// Partial configuration. Defaults belong to creation, never to omitted patch fields.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebhookPatch {
    #[serde(default, skip_serializing_if = "crate::FieldUpdate::is_missing")]
    pub enabled: crate::FieldUpdate<bool>,
    #[serde(default, skip_serializing_if = "crate::FieldUpdate::is_missing")]
    pub provider: crate::FieldUpdate<WebhookProvider>,
    #[serde(default, skip_serializing_if = "crate::FieldUpdate::is_missing")]
    pub auth_scheme: crate::FieldUpdate<WebhookAuthScheme>,
    #[serde(default, skip_serializing_if = "crate::FieldUpdate::is_missing")]
    pub secret: crate::FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "crate::FieldUpdate::is_missing")]
    pub branch_filter: crate::FieldUpdate<Option<String>>,
}
impl WebhookPatch {
    pub fn apply(self, current: WebhookConfig) -> WebhookConfig {
        WebhookConfig {
            enabled: self.enabled.apply(current.enabled),
            provider: self.provider.apply(current.provider),
            auth_scheme: self.auth_scheme.apply(current.auth_scheme),
            secret: self.secret.apply(current.secret),
            branch_filter: self.branch_filter.apply(current.branch_filter),
        }
    }
}
#[cfg(test)]
mod patch_tests {
    use super::*;
    #[test]
    fn rotating_secret_preserves_enabled_provider_and_filter() {
        let current = WebhookConfig {
            enabled: true,
            provider: WebhookProvider::Generic,
            auth_scheme: WebhookAuthScheme::BearerToken,
            secret: Some("old".into()),
            branch_filter: Some("main".into()),
        };
        let patch: WebhookPatch = serde_json::from_str(r#"{"secret":"new"}"#).unwrap();
        let value = patch.apply(current);
        assert!(value.enabled);
        assert_eq!(value.provider, WebhookProvider::Generic);
        assert_eq!(value.secret.as_deref(), Some("new"));
        assert_eq!(value.branch_filter.as_deref(), Some("main"));
        value.validate().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn supported_provider_scheme_pairs_are_explicit() {
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
                let config = WebhookConfig {
                    enabled: true,
                    provider,
                    auth_scheme,
                    secret: Some("key".into()),
                    branch_filter: None,
                };
                let expected = matches!(
                    (provider, auth_scheme),
                    (WebhookProvider::GitHub, WebhookAuthScheme::GitHubHmacSha256)
                        | (
                            WebhookProvider::GitLab,
                            WebhookAuthScheme::GitLabSignedToken
                                | WebhookAuthScheme::GitLabLegacyToken
                        )
                        | (WebhookProvider::Generic, WebhookAuthScheme::BearerToken)
                );
                assert_eq!(config.validate().is_ok(), expected);
            }
        }
    }

    #[test]
    fn invalid_fields_and_empty_bearer_secrets_are_rejected() {
        for value in [
            json!({"enabled":"true"}),
            json!({"provider":"Unknown"}),
            json!({"authScheme":"Unknown"}),
            json!({"secret":42}),
        ] {
            assert!(serde_json::from_value::<WebhookConfig>(value).is_err());
        }
        for secret in [
            None,
            Some("".into()),
            Some("  ".into()),
            Some("x".repeat(257)),
        ] {
            let config = WebhookConfig {
                enabled: true,
                provider: WebhookProvider::Generic,
                auth_scheme: WebhookAuthScheme::BearerToken,
                secret,
                branch_filter: None,
            };
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn patch_distinguishes_missing_and_cleared_credentials() {
        let config = WebhookConfig {
            secret: Some("original".into()),
            branch_filter: Some("main".into()),
            ..Default::default()
        };
        let patch: WebhookPatch = serde_json::from_value(json!({})).unwrap();
        assert_eq!(patch.apply(config.clone()), config);
        let patch: WebhookPatch = serde_json::from_value(json!({"secret":null})).unwrap();
        let cleared = patch.apply(config);
        assert_eq!(cleared.secret, None);
        assert_eq!(cleared.branch_filter.as_deref(), Some("main"));
    }

    #[test]
    fn audit_redaction_keeps_configuration_without_exposing_credentials() {
        let config = WebhookConfig {
            secret: Some("private-key".into()),
            branch_filter: Some("main".into()),
            ..Default::default()
        };
        let redacted = config.redacted();
        assert_eq!(redacted.secret.as_deref(), Some("********"));
        assert_eq!(redacted.branch_filter, config.branch_filter);
        assert_eq!(config.secret.as_deref(), Some("private-key"));
        assert_eq!(WebhookConfig::default().redacted().secret, None);
    }
}
