use std::sync::Arc;
use std::time::Duration;

use citadel_bindings::BindingError;
use citadel_bindings::SecretProtector;
use citadel_bindings::SecretProviderTester;
use citadel_bindings::SecretTestResult;
use citadel_bindings::TestExternalSecretInput;
use citadel_bindings::TestSecretProviderInput;
use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use reqwest::redirect::Policy;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

const MAX_PROVIDER_RESPONSE_BYTES: usize = 1024 * 1024;

impl SecretProviderTester for PostgresSecretValueResolver {
    fn test_connection<'a>(
        &'a self,
        input: &'a TestSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretTestResult, BindingError>> {
        Box::pin(async move {
            input.validate()?;
            let existing = match input.provider_id {
                Some(id) => Some(self.provider_configuration(id).await?),
                None => None,
            };
            let supplied = input
                .token
                .as_deref()
                .filter(|token| !token.trim().is_empty());
            let stored;
            let token = if let Some(token) = supplied {
                token
            } else if let Some(config) = existing.as_ref() {
                stored = match self.provider_token(config) {
                    Ok(token) => token,
                    Err(_) => {
                        return Ok(SecretTestResult::new(
                            false,
                            "Secret provider token could not be decrypted.",
                        ));
                    }
                };
                &stored
            } else {
                return Ok(SecretTestResult::new(
                    false,
                    "Vault token is required to test the connection.",
                ));
            };
            let mut url = provider_url(&input.address)
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
                    if supplied.is_some() {
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
        })
    }

    fn test_external<'a>(
        &'a self,
        input: &'a TestExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretTestResult, BindingError>> {
        Box::pin(async move {
            input.validate()?;
            let config = self.provider_configuration(input.provider_id).await?;
            let token = match self.provider_token(&config) {
                Ok(token) => token,
                Err(_) => {
                    return Ok(SecretTestResult::new(
                        false,
                        "Secret provider token could not be decrypted.",
                    ));
                }
            };
            let result = self
                .resolve_vault(VaultSecretRequest {
                    name: "EXTERNAL_SECRET_TEST",
                    address: json_string(&config, &["Address", "address"]).unwrap_or_default(),
                    mount: json_string(&config, &["MountPath", "mountPath"]).unwrap_or_default(),
                    path: &input.external_path,
                    key: &input.external_key,
                    version: input.external_version,
                    token: &token,
                })
                .await;
            // Never return provider response bodies, tokens, or the resolved value.
            Ok(SecretTestResult::new(
                result.is_ok(),
                if result.is_ok() {
                    "External secret reference resolved successfully."
                } else {
                    "External secret reference could not be resolved. Check the provider, path, key, version and token permissions."
                },
            ))
        })
    }
}

#[derive(Clone)]
pub struct PostgresSecretValueResolver {
    pool: PgPool,
    protector: Arc<dyn SecretProtector>,
    client: reqwest::Client,
}

struct VaultSecretRequest<'a> {
    name: &'a str,
    address: &'a str,
    mount: &'a str,
    path: &'a str,
    key: &'a str,
    version: Option<i32>,
    token: &'a str,
}

impl PostgresSecretValueResolver {
    async fn provider_configuration(&self, id: Uuid) -> Result<Value, BindingError> {
        let row = sqlx::query("SELECT providertype,configuration FROM secretproviders WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| BindingError::Storage(e.to_string()))?
            .ok_or(BindingError::NotFound)?;
        let kind: String = row
            .try_get("providertype")
            .map_err(|e| BindingError::Storage(e.to_string()))?;
        if kind != "VaultCompatibleKvV2" {
            return Err(BindingError::Validation(
                "The Secret provider is not supported.".into(),
            ));
        }
        let configuration: String = row
            .try_get("configuration")
            .map_err(|e| BindingError::Storage(e.to_string()))?;
        serde_json::from_str(&configuration).map_err(|_| BindingError::Credential)
    }

    fn provider_token(&self, config: &Value) -> Result<Zeroizing<String>, SecretValueError> {
        self.decrypt(
            json_string(config, &["ProtectedToken", "protectedToken"]).unwrap_or_default(),
            "provider token",
        )
    }

    pub fn new(
        pool: PgPool,
        protector: Arc<dyn SecretProtector>,
    ) -> Result<Self, SecretValueError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(Policy::none())
            .build()
            .map_err(|error| SecretValueError::External(error.to_string()))?;
        Ok(Self {
            pool,
            protector,
            client,
        })
    }

    pub async fn resolve(&self, secret_id: Uuid) -> Result<Zeroizing<String>, SecretValueError> {
        let row = sqlx::query(
            r#"SELECT secret.name,secret.providertype,secret.externalpath,secret.externalkey,
                      secret.externalversion,internal.encryptedvalue,
                      provider.providertype AS externalprovidertype,provider.configuration
               FROM secretdefinitions secret
               LEFT JOIN internalsecretvalues internal ON internal.secretid=secret.id
               LEFT JOIN secretproviders provider ON provider.id=secret.providerid
               WHERE secret.id=$1"#,
        )
        .bind(secret_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| SecretValueError::Validation("Secret was not found.".into()))?;
        let name: String = row.try_get("name").map_err(storage)?;
        match row
            .try_get::<String, _>("providertype")
            .map_err(storage)?
            .as_str()
        {
            "InternalEncrypted" => self.decrypt(
                row.try_get::<Option<String>, _>("encryptedvalue")
                    .map_err(storage)?
                    .as_deref()
                    .ok_or_else(|| {
                        SecretValueError::Validation(format!(
                            "Secret '{name}' has no encrypted value."
                        ))
                    })?,
                &name,
            ),
            "VaultCompatibleKvV2" => {
                if row
                    .try_get::<Option<String>, _>("externalprovidertype")
                    .map_err(storage)?
                    .as_deref()
                    != Some("VaultCompatibleKvV2")
                {
                    return Err(SecretValueError::Validation(format!(
                        "Secret '{name}' has no compatible provider."
                    )));
                }
                let configuration = serde_json::from_str::<Value>(
                    &row.try_get::<String, _>("configuration").map_err(storage)?,
                )
                .map_err(|_| provider_configuration(&name))?;
                let address = json_string(&configuration, &["Address", "address"])
                    .ok_or_else(|| provider_configuration(&name))?;
                let mount = json_string(&configuration, &["MountPath", "mountPath"])
                    .ok_or_else(|| provider_configuration(&name))?;
                let protected_token =
                    json_string(&configuration, &["ProtectedToken", "protectedToken"])
                        .ok_or_else(|| provider_configuration(&name))?;
                let token = self.decrypt(protected_token, &name)?;
                let path: String = row
                    .try_get::<Option<String>, _>("externalpath")
                    .map_err(storage)?
                    .ok_or_else(|| provider_configuration(&name))?;
                let key: String = row
                    .try_get::<Option<String>, _>("externalkey")
                    .map_err(storage)?
                    .ok_or_else(|| provider_configuration(&name))?;
                let version: Option<i32> = row.try_get("externalversion").map_err(storage)?;
                self.resolve_vault(VaultSecretRequest {
                    name: &name,
                    address,
                    mount,
                    path: &path,
                    key: &key,
                    version,
                    token: &token,
                })
                .await
            }
            provider => Err(SecretValueError::Validation(format!(
                "Secret '{name}' uses unsupported provider '{provider}'."
            ))),
        }
    }

    fn decrypt(&self, envelope: &str, name: &str) -> Result<Zeroizing<String>, SecretValueError> {
        let bytes = self.protector.unprotect(envelope).map_err(|_| {
            SecretValueError::Validation(format!("Secret '{name}' could not be decrypted."))
        })?;
        String::from_utf8(bytes.to_vec())
            .map(Zeroizing::new)
            .map_err(|_| {
                SecretValueError::Validation(format!(
                    "Secret '{name}' does not contain valid UTF-8 text."
                ))
            })
    }

    async fn resolve_vault(
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
    let url = reqwest::Url::parse(address)
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

fn json_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
}

fn provider_configuration(name: &str) -> SecretValueError {
    SecretValueError::Validation(format!(
        "Secret '{name}' provider configuration is incomplete."
    ))
}

fn storage(error: sqlx::Error) -> SecretValueError {
    SecretValueError::Storage(error.to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum SecretValueError {
    #[error("{0}")]
    Validation(String),
    #[error("secret storage failed: {0}")]
    Storage(String),
    #[error("external Secret provider failed: {0}")]
    External(String),
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
