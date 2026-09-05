use std::sync::Arc;
use std::time::Duration;

use citadel_resources::ResourceSecretProtector;
use futures_util::StreamExt;
use reqwest::redirect::Policy;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

const MAX_PROVIDER_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct PostgresSecretValueResolver {
    pool: PgPool,
    protector: Arc<dyn ResourceSecretProtector>,
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
    pub fn new(
        pool: PgPool,
        protector: Arc<dyn ResourceSecretProtector>,
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
        let mut url = reqwest::Url::parse(request.address)
            .map_err(|_| SecretValueError::Validation("Secret provider URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(SecretValueError::Validation(
                "Secret provider URL is invalid.".into(),
            ));
        }
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
}
