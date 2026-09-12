use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ResourceMetadataError, validate_provider};

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestSecretProviderInput {
    pub provider_id: Option<Uuid>,
    pub name: Option<String>,
    pub address: String,
    pub mount_path: String,
    pub token: Option<String>,
}

impl TestSecretProviderInput {
    pub fn validate(&self) -> Result<(), ResourceMetadataError> {
        if self.address.len() > 512 || self.mount_path.len() > 128 {
            return Err(ResourceMetadataError::Validation(
                "Provider address or mount path is too long.".into(),
            ));
        }
        validate_provider(
            self.name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("Vault provider"),
            &self.address,
            &self.mount_path,
        )
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestExternalSecretInput {
    pub provider_id: Uuid,
    pub external_path: String,
    pub external_key: String,
    pub external_version: Option<i32>,
}

impl TestExternalSecretInput {
    pub fn validate(&self) -> Result<(), ResourceMetadataError> {
        crate::bindings::validate_external_secret(
            "EXTERNAL_SECRET_TEST",
            self.provider_id,
            &self.external_path,
            &self.external_key,
            self.external_version,
        )
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretTestResult {
    pub success: bool,
    pub message: String,
}

impl SecretTestResult {
    pub fn new(success: bool, message: impl Into<String>) -> Self {
        Self {
            success,
            message: message.into(),
        }
    }
}

pub trait SecretProviderTester: Send + Sync {
    fn test_connection<'a>(
        &'a self,
        input: &'a TestSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretTestResult, ResourceMetadataError>>;
    fn test_external<'a>(
        &'a self,
        input: &'a TestExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretTestResult, ResourceMetadataError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_tests_use_the_same_validation_as_stored_secrets() {
        for (id, path, key, version) in [
            (Uuid::nil(), "app", "key", None),
            (Uuid::now_v7(), "", "key", None),
            (Uuid::now_v7(), "app", "", None),
            (Uuid::now_v7(), "app", "key", Some(0)),
        ] {
            assert!(
                TestExternalSecretInput {
                    provider_id: id,
                    external_path: path.into(),
                    external_key: key.into(),
                    external_version: version
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            TestExternalSecretInput {
                provider_id: Uuid::now_v7(),
                external_path: "app".into(),
                external_key: "key".into(),
                external_version: Some(1)
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn connection_test_does_not_require_a_new_token_but_bounds_configuration() {
        let mut input: TestSecretProviderInput = serde_json::from_str(
            r#"{"address":"https://vault.example.test","mountPath":"secret"}"#,
        )
        .unwrap();
        assert!(input.validate().is_ok());
        input.mount_path = "x".repeat(129);
        assert!(input.validate().is_err());
    }
}
