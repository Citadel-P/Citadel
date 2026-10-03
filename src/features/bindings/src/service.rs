use crate::*;
use std::sync::Arc;
use uuid::Uuid;

pub struct SecretService {
    pub(crate) store: Arc<dyn BindingRepository>,
    pub(crate) protector: Arc<dyn SecretProtector>,
    pub(crate) provider_tester: Option<Arc<dyn SecretProviderTester>>,
}

impl SecretService {
    #[must_use]
    pub fn new(store: Arc<dyn BindingRepository>, protector: Arc<dyn SecretProtector>) -> Self {
        Self {
            store,
            protector,
            provider_tester: None,
        }
    }

    pub fn with_secret_provider_tester(mut self, tester: Arc<dyn SecretProviderTester>) -> Self {
        self.provider_tester = Some(tester);
        self
    }

    pub async fn test_secret_provider(
        &self,
        input: TestSecretProviderInput,
    ) -> Result<SecretTestResult, BindingError> {
        input.validate()?;
        self.provider_tester
            .as_ref()
            .ok_or_else(|| {
                BindingError::Storage("Secret provider testing is not configured.".into())
            })?
            .test_connection(&input)
            .await
    }

    pub async fn test_external_secret(
        &self,
        input: TestExternalSecretInput,
    ) -> Result<SecretTestResult, BindingError> {
        input.validate()?;
        self.provider_tester
            .as_ref()
            .ok_or_else(|| {
                BindingError::Storage("Secret provider testing is not configured.".into())
            })?
            .test_external(&input)
            .await
    }

    #[must_use]
    pub fn store(&self) -> &Arc<dyn BindingRepository> {
        &self.store
    }

    pub async fn create_internal_secret(
        &self,
        name: &str,
        value: &str,
    ) -> Result<SecretDefinition, BindingError> {
        validate_name(name, "Secret")?;
        if value.is_empty() || value.len() > 256 * 1024 {
            return Err(BindingError::Validation(
                "Secret value must contain between 1 and 262144 bytes.".to_owned(),
            ));
        }
        let protected = self.protector.protect(value.as_bytes())?;
        self.store
            .create_internal_secret(name.trim(), &protected)
            .await
    }

    pub async fn create_secret_provider(
        &self,
        input: SecretProviderInput,
    ) -> Result<SecretProvider, BindingError> {
        validate_provider(&input.name, &input.address, &input.mount_path)?;
        if input.token.is_empty() {
            return Err(BindingError::Validation(
                "Secret provider token is required.".to_owned(),
            ));
        }
        let stored = StoredSecretProviderInput {
            name: input.name.trim().to_owned(),
            address: input.address.trim_end_matches('/').to_owned(),
            mount_path: input.mount_path.trim_matches('/').to_owned(),
            protected_token: self.protector.protect(input.token.as_bytes())?,
        };
        self.store.create_secret_provider(&stored).await
    }

    pub async fn update_secret_provider(
        &self,
        id: Uuid,
        input: SecretProviderPatch,
    ) -> Result<SecretProvider, BindingError> {
        let current = self.store.get_secret_provider(id).await?;
        validate_provider(
            input.name.as_deref().unwrap_or(&current.name),
            input.address.as_deref().unwrap_or(&current.address),
            input.mount_path.as_deref().unwrap_or(&current.mount_path),
        )?;
        let stored = StoredSecretProviderPatch {
            name: input.name.map(|value| value.trim().to_owned()),
            address: input
                .address
                .map(|value| value.trim().trim_end_matches('/').to_owned()),
            mount_path: input
                .mount_path
                .map(|value| value.trim_matches('/').to_owned()),
            protected_token: input
                .token
                .filter(|value| !value.trim().is_empty())
                .map(|value| self.protector.protect(value.as_bytes()))
                .transpose()?,
        };
        self.store.update_secret_provider(id, &stored).await
    }
}
