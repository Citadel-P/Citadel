use citadel_bindings::{BindingError, SecretTestResult, SecretValueError};
use futures_util::StreamExt;
use reqwest::redirect::Policy;
use serde_json::Value;
use std::time::Duration;
use zeroize::Zeroizing;

const MAX_PROVIDER_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct VaultClient {
    client: reqwest::Client,
}

pub(crate) struct VaultSecretRequest<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub mount: &'a str,
    pub path: &'a str,
    pub key: &'a str,
    pub version: Option<i32>,
    pub token: &'a str,
}

impl VaultClient {
    pub(crate) fn new() -> Result<Self, SecretValueError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .build()
            .map_err(|error| SecretValueError::External(error.to_string()))?;
        Ok(Self { client })
    }
    pub(crate) async fn test_connection(
        &self,
        address: &str,
        token: &str,
        supplied: bool,
    ) -> Result<SecretTestResult, BindingError> {
        let mut url = provider_url(address)
            .map_err(|_| BindingError::Validation("Secret provider URL is invalid.".into()))?;
        let mut health_url = url.clone();
        health_url
            .path_segments_mut()
            .map_err(|_| BindingError::Validation("Secret provider URL is invalid.".into()))?
            .pop_if_empty()
            .extend(["v1", "sys", "health"]);
        let health = match self.client.get(health_url).send().await {
            Ok(response) => response.status().as_u16(),
            Err(_) => {
                return Ok(SecretTestResult::new(
                    false,
                    "Vault could not be reached within the connection timeout.",
                ));
            }
        };
        if health >= 500 {
            return Ok(SecretTestResult::new(
                false,
                format!("Vault endpoint is reachable but not ready: HTTP {health}."),
            ));
        }
        url.path_segments_mut()
            .map_err(|_| BindingError::Validation("Secret provider URL is invalid.".into()))?
            .pop_if_empty()
            .extend(["v1", "auth", "token", "lookup-self"]);
        let response = self
            .client
            .get(url)
            .header("X-Vault-Token", token)
            .send()
            .await;
        Ok(match response {
            Ok(response) if response.status().is_success() => SecretTestResult::new(
                true,
                if supplied {
                    "Connection successful. Vault is reachable and the token is valid."
                } else {
                    "Connection successful using the stored token. Vault is reachable and the token is valid."
                },
            ),
            Ok(response)
                if matches!(response.status().as_u16(), 404 | 405)
                    && matches!(health, 200 | 429 | 472 | 473) =>
            {
                SecretTestResult::new(
                    true,
                    "Vault is reachable. Token lookup is not supported by this provider; test a secret reference to verify token and KV access.",
                )
            }
            Ok(response) => SecretTestResult::new(
                false,
                format!(
                    "Vault connection test failed: HTTP {}.",
                    response.status().as_u16()
                ),
            ),
            Err(_) => SecretTestResult::new(
                false,
                "Vault could not be reached within the connection timeout.",
            ),
        })
    }
    pub(crate) async fn resolve(
        &self,
        request: VaultSecretRequest<'_>,
    ) -> Result<Zeroizing<String>, SecretValueError> {
        let mut url = provider_url(request.address)?;
        let mount = safe_segments(request.mount)?;
        let path = safe_segments(request.path)?;
        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                SecretValueError::Validation("Secret provider URL is invalid.".into())
            })?;
            segments.pop_if_empty().push("v1");
            for segment in mount {
                segments.push(segment);
            }
            segments.push("data");
            for segment in path {
                segments.push(segment);
            }
        }
        if let Some(version) = request.version {
            url.query_pairs_mut()
                .append_pair("version", &version.to_string());
        }
        let response = self
            .client
            .get(url)
            .header("X-Vault-Token", request.token)
            .send()
            .await
            .map_err(|error| {
                SecretValueError::External(format!(
                    "Secret '{}' could not be resolved: {error}",
                    request.name
                ))
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(SecretValueError::External(format!(
                "Secret '{}' could not be resolved: HTTP {}.",
                request.name,
                status.as_u16()
            )));
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| SecretValueError::External(error.to_string()))?;
            if body.len().saturating_add(chunk.len()) > MAX_PROVIDER_RESPONSE_BYTES {
                return Err(SecretValueError::External(format!(
                    "Secret '{}' provider response exceeded 1 MiB.",
                    request.name
                )));
            }
            body.extend_from_slice(&chunk);
        }
        let payload: Value = serde_json::from_slice(&body).map_err(|_| {
            SecretValueError::External(format!(
                "Secret '{}' provider returned invalid JSON.",
                request.name
            ))
        })?;
        let value = payload
            .pointer("/data/data")
            .and_then(Value::as_object)
            .and_then(|values| values.get(request.key))
            .ok_or_else(|| {
                SecretValueError::External(format!(
                    "Secret '{}' key '{}' was not found in the provider response.",
                    request.name, request.key
                ))
            })?;
        Ok(Zeroizing::new(match value {
            Value::String(value) => value.clone(),
            value => serde_json::to_string(value).map_err(|error| {
                SecretValueError::External(format!(
                    "Secret '{}' value could not be decoded: {error}",
                    request.name
                ))
            })?,
        }))
    }
}
fn provider_url(address: &str) -> Result<reqwest::Url, SecretValueError> {
    let url = reqwest::Url::parse(address.trim().trim_end_matches('/'))
        .map_err(|_| SecretValueError::Validation("Secret provider URL is invalid.".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(SecretValueError::Validation(
            "Secret provider URL is invalid.".into(),
        ));
    }
    Ok(url)
}

fn safe_segments(value: &str) -> Result<Vec<&str>, SecretValueError> {
    let segments = value
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| matches!(*segment, "." | ".."))
    {
        Err(SecretValueError::Validation(
            "Secret provider path is invalid.".into(),
        ))
    } else {
        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_paths_reject_traversal() {
        assert_eq!(safe_segments("apps/citadel").unwrap(), ["apps", "citadel"]);
        assert!(safe_segments("apps/../admin").is_err());
    }

    #[test]
    fn provider_urls_reject_embedded_credentials_and_non_http_targets() {
        for address in [
            "file:///etc/passwd",
            "https://user:password@vault.test",
            "https://vault.test?token=secret",
            "https://vault.test#fragment",
        ] {
            assert!(provider_url(address).is_err());
        }
        assert!(provider_url("https://vault.test/prefix").is_ok());
    }
}
