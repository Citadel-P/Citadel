use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

mod config;
pub use config::{RepoWebhookConfig, WebhookAuthScheme, WebhookProvider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookConfiguration {
    pub enabled: bool,
    pub provider: String,
    pub auth_scheme: String,
    pub secret: Option<String>,
    pub branch_filter: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("Webhook authentication failed.")]
    Authentication,
    #[error("{0}")]
    Validation(String),
}

impl WebhookConfiguration {
    pub fn from_value(value: Option<&serde_json::Value>) -> Result<Option<Self>, WebhookError> {
        let Some(value) = value else { return Ok(None) };
        crate::validate_webhook(Some(value))
            .map_err(|error| WebhookError::Validation(error.to_string()))?;
        let field = |name: &str| {
            value.as_object().and_then(|fields| {
                fields
                    .iter()
                    .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
            })
        };
        let enabled = field("enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !enabled {
            return Ok(None);
        }
        Ok(Some(Self {
            enabled,
            provider: field("provider")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("GitHub")
                .into(),
            auth_scheme: field("authScheme")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("GitHubHmacSha256")
                .into(),
            secret: field("secret")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            branch_filter: field("branchFilter")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        }))
    }

    /// Authenticate before parsing or dispatching. A route names the provider,
    /// but never selects a weaker scheme than the resource's saved configuration.
    pub fn evaluate(
        &self,
        auth_type: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<Option<&'static str>, WebhookError> {
        if body.len() > 1024 * 1024 {
            return Err(WebhookError::Validation(
                "Webhook payload exceeds the 1 MiB limit.".into(),
            ));
        }
        if !self.provider.eq_ignore_ascii_case(auth_type) {
            return Err(WebhookError::Validation(
                "Webhook auth type does not match the configured provider.".into(),
            ));
        }
        authenticate_webhook(self, headers, body)?;
        let (supported, branch) = webhook_branch(&self.provider, headers, body)?;
        if !supported {
            return Ok(Some("Unsupported event type"));
        }
        if let Some(filter) = self
            .branch_filter
            .as_deref()
            .map(str::trim)
            .filter(|filter| !filter.is_empty())
            && branch.as_deref() != Some(filter)
            && (branch.is_some() || !self.provider.eq_ignore_ascii_case("Generic"))
        {
            return Ok(Some("Branch filter did not match"));
        }
        Ok(None)
    }
}

pub fn authenticate_webhook(
    webhook: &WebhookConfiguration,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(), WebhookError> {
    let secret = webhook.secret.as_deref().unwrap_or_default();
    let authenticated = match webhook.auth_scheme.as_str() {
        "BearerToken" if !secret.trim().is_empty() => header(headers, "authorization")
            .and_then(|value| {
                value.split_once(' ').and_then(|(scheme, token)| {
                    scheme.eq_ignore_ascii_case("Bearer").then_some(token)
                })
            })
            .is_some_and(|value| fixed_time(value.trim().as_bytes(), secret.as_bytes())),
        "GitLabLegacyToken" if !secret.is_empty() => header(headers, "x-gitlab-token")
            .is_some_and(|value| fixed_time(value.as_bytes(), secret.as_bytes())),
        "GitLabSignedToken" => validate_gitlab_signed(headers, body, secret),
        "GitHubHmacSha256" if secret.is_empty() => true,
        "GitHubHmacSha256" => {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                .map_err(|_| WebhookError::Authentication)?;
            mac.update(body);
            let expected = hex_lower(&mac.finalize().into_bytes());
            header(headers, "x-hub-signature-256")
                .and_then(|value| value.strip_prefix("sha256="))
                .or_else(|| header(headers, "x-gitea-signature"))
                .or_else(|| header(headers, "x-forgejo-signature"))
                .is_some_and(|value| fixed_time(value.as_bytes(), expected.as_bytes()))
        }
        _ => false,
    };
    if authenticated {
        Ok(())
    } else {
        Err(WebhookError::Authentication)
    }
}

pub fn webhook_branch(
    provider: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(bool, Option<String>), WebhookError> {
    if provider.eq_ignore_ascii_case("Generic") {
        let payload = serde_json::from_slice::<serde_json::Value>(body).ok();
        let branch = payload
            .as_ref()
            .and_then(|value| value.get("branch"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        return Ok((true, branch));
    }
    if provider.eq_ignore_ascii_case("GitHub") {
        let event = header(headers, "x-github-event")
            .or_else(|| header(headers, "x-gitea-event"))
            .or_else(|| header(headers, "x-forgejo-event"));
        if event.is_some_and(|event| !event.eq_ignore_ascii_case("push")) {
            return Ok((false, None));
        }
    }
    let payload: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| WebhookError::Validation("Webhook payload must be valid JSON.".to_owned()))?;
    if payload.get("deleted").and_then(serde_json::Value::as_bool) == Some(true) {
        return Ok((false, None));
    }
    if provider.eq_ignore_ascii_case("GitLab")
        && header(headers, "x-gitlab-event")
            .is_some_and(|event| !event.eq_ignore_ascii_case("Push Hook"))
    {
        return Ok((false, None));
    }
    let reference = payload
        .get("ref")
        .and_then(serde_json::Value::as_str)
        .or_else(|| payload.get("ref_name").and_then(serde_json::Value::as_str));
    Ok((
        true,
        reference.map(|value| {
            value
                .strip_prefix("refs/heads/")
                .unwrap_or(value)
                .to_owned()
        }),
    ))
}

fn validate_gitlab_signed(headers: &[(String, String)], body: &[u8], secret: &str) -> bool {
    let Some(webhook_id) = header(headers, "webhook-id") else {
        return false;
    };
    let Some(timestamp) = header(headers, "webhook-timestamp") else {
        return false;
    };
    let Some(signatures) = header(headers, "webhook-signature") else {
        return false;
    };
    let Ok(timestamp_value) = timestamp.parse::<i64>() else {
        return false;
    };
    if Utc::now().timestamp().abs_diff(timestamp_value) > 5 * 60 {
        return false;
    }
    let Some(encoded_key) = secret.strip_prefix("whsec_") else {
        return false;
    };
    let Ok(key) = STANDARD.decode(encoded_key) else {
        return false;
    };
    let mut mac = match Hmac::<Sha256>::new_from_slice(&key) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(format!("{webhook_id}.{timestamp}.").as_bytes());
    mac.update(body);
    let expected = format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()));
    signatures
        .split_ascii_whitespace()
        .any(|candidate| fixed_time(candidate.as_bytes(), expected.as_bytes()))
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value.as_str()))
}

fn fixed_time(actual: &[u8], expected: &[u8]) -> bool {
    actual.len() == expected.len() && bool::from(actual.ct_eq(expected))
}

/// Match only identity fields supplied by the provider; an absent identity is
/// allowed for generic CI triggers authenticated with the resource's secret.
pub fn repository_matches(url: &str, payload: &serde_json::Value) -> bool {
    let normalize = |value: &str| {
        value
            .trim()
            .trim_end_matches('/')
            .to_ascii_lowercase()
            .trim_end_matches(".git")
            .to_owned()
    };
    let expected = normalize(url);
    let repository = payload.get("repository").or_else(|| payload.get("project"));
    let candidates = repository
        .and_then(serde_json::Value::as_str)
        .into_iter()
        .chain(
            [
                "html_url",
                "clone_url",
                "ssh_url",
                "git_http_url",
                "git_ssh_url",
            ]
            .into_iter()
            .filter_map(|field| {
                repository
                    .and_then(|value| value.get(field))
                    .and_then(serde_json::Value::as_str)
            }),
        )
        .filter(|value| !value.trim().is_empty());
    let full_name = repository
        .and_then(|value| {
            value
                .get("full_name")
                .or_else(|| value.get("path_with_namespace"))
        })
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let mut supplied = false;
    for candidate in candidates {
        supplied = true;
        if normalize(candidate) == expected {
            return true;
        }
    }
    if let Some(name) = full_name {
        return expected.ends_with(&format!("/{}", normalize(name)));
    }
    !supplied
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[(byte >> 4) as usize]));
        output.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    fn webhook(provider: &str, scheme: &str, secret: &str) -> WebhookConfiguration {
        WebhookConfiguration {
            enabled: true,
            provider: provider.to_owned(),
            auth_scheme: scheme.to_owned(),
            secret: Some(secret.to_owned()),
            branch_filter: Some("main".to_owned()),
        }
    }

    #[test]
    fn webhook_authentication_supports_bearer_and_provider_hmac_without_path_authority() {
        let generic = webhook("Generic", "BearerToken", "shared-secret");
        assert!(
            authenticate_webhook(
                &generic,
                &[(
                    "Authorization".to_owned(),
                    "Bearer shared-secret".to_owned()
                )],
                b"{}"
            )
            .is_ok()
        );
        assert!(authenticate_webhook(&generic, &[], b"{}").is_err());

        let github = webhook("GitHub", "GitHubHmacSha256", "secret");
        let body = br#"{"ref":"refs/heads/main"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex_lower(&mac.finalize().into_bytes()));
        assert!(
            authenticate_webhook(
                &github,
                &[("X-Hub-Signature-256".to_owned(), signature)],
                body
            )
            .is_ok()
        );
        assert!(
            authenticate_webhook(
                &github,
                &[("X-Hub-Signature-256".to_owned(), "sha256=bad".to_owned())],
                body
            )
            .is_err()
        );
    }

    #[test]
    fn webhook_branch_is_extracted_without_accepting_invalid_json() {
        assert_eq!(
            webhook_branch(
                "GitHub",
                &[("X-GitHub-Event".to_owned(), "push".to_owned())],
                br#"{"ref":"refs/heads/release"}"#
            )
            .unwrap()
            .1
            .as_deref(),
            Some("release")
        );
        assert!(webhook_branch("GitHub", &[], b"not-json").is_err());
    }

    #[test]
    fn generic_branch_filter_authenticates_even_ignored_events() {
        let config = webhook("Generic", "BearerToken", "key");
        let headers = [("Authorization".into(), "bEaReR key".into())];
        assert_eq!(config.evaluate("generic", &headers, b"{}").unwrap(), None);
        assert_eq!(
            config
                .evaluate("generic", &headers, br#"{"branch":"main"}"#)
                .unwrap(),
            None
        );
        assert!(
            config
                .evaluate("generic", &headers, br#"{"branch":"other"}"#)
                .unwrap()
                .is_some()
        );
        assert!(
            config
                .evaluate("generic", &[], br#"{"branch":"other"}"#)
                .is_err()
        );
        assert!(config.evaluate("github", &headers, b"{}").is_err());
        assert!(
            webhook("Generic", "BearerToken", "")
                .evaluate(
                    "generic",
                    &[("authorization".into(), "Bearer ".into())],
                    b"{}"
                )
                .is_err()
        );
    }

    #[test]
    fn provider_events_do_not_dispatch_deleted_branches_tags_or_non_push_events() {
        let config = webhook("GitHub", "GitHubHmacSha256", "");
        for body in [
            br#"{"ref":"refs/heads/main","deleted":true}"#.as_slice(),
            br#"{"ref":"refs/tags/main"}"#,
            b"{}",
        ] {
            assert!(config.evaluate("github", &[], body).unwrap().is_some());
        }
        let config = webhook("GitLab", "GitLabLegacyToken", "key");
        assert!(
            config
                .evaluate(
                    "gitlab",
                    &[
                        ("x-gitlab-token".into(), "key".into()),
                        ("x-gitlab-event".into(), "Merge Request Hook".into())
                    ],
                    br#"{"ref":"refs/heads/main"}"#
                )
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn signed_gitlab_token_rejects_tampering_and_extreme_or_stale_timestamps() {
        let key = b"test-only-signing-key";
        let config = webhook(
            "GitLab",
            "GitLabSignedToken",
            &format!("whsec_{}", STANDARD.encode(key)),
        );
        let body = br#"{"ref":"refs/heads/main"}"#;
        for timestamp in [
            Utc::now().timestamp(),
            Utc::now().timestamp() - 301,
            Utc::now().timestamp() + 301,
            i64::MIN,
            i64::MAX,
        ] {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
            mac.update(format!("event-id.{timestamp}.").as_bytes());
            mac.update(body);
            let headers = [
                ("webhook-id".into(), "event-id".into()),
                ("webhook-timestamp".into(), timestamp.to_string()),
                (
                    "webhook-signature".into(),
                    format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes())),
                ),
            ];
            assert_eq!(
                authenticate_webhook(&config, &headers, body).is_ok(),
                Utc::now().timestamp().abs_diff(timestamp) <= 300
            );
            assert!(authenticate_webhook(&config, &headers, b"tampered").is_err());
        }
    }

    #[test]
    fn provider_repository_identity_must_match_without_suffix_confusion() {
        for payload in [
            serde_json::json!({"repository":{"full_name":"team/repo"}}),
            serde_json::json!({"project":{"path_with_namespace":"team/repo"}}),
            serde_json::json!({"repository":"https://example.test/team/repo.git"}),
        ] {
            assert!(repository_matches(
                "https://example.test/team/repo.git",
                &payload
            ));
            assert!(!repository_matches(
                "https://example.test/other-team/repo.git",
                &payload
            ));
        }
    }
}
