use std::sync::Arc;

use citadel_bindings::BindingError;
use citadel_bindings::SecretProtector;
use citadel_bindings::SecretProviderTester;
use citadel_bindings::SecretTestResult;
use citadel_bindings::TestExternalSecretInput;
use citadel_bindings::TestSecretProviderInput;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::connectors::vault::{VaultClient, VaultSecretRequest};
pub use citadel_bindings::SecretValueError;

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
            if let Some(config) = existing.as_ref() {
                citadel_bindings::validate_provider_token_reuse(
                    json_string(config, &["Address", "address"]).unwrap_or_default(),
                    &input.address,
                    supplied.is_some(),
                )?;
            }
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
            self.client
                .test_connection(&input.address, token, supplied.is_some())
                .await
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
                .client
                .resolve(VaultSecretRequest {
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
    client: VaultClient,
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
        let client = VaultClient::new()?;
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
                self.client
                    .resolve(VaultSecretRequest {
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
