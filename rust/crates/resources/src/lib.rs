#![forbid(unsafe_code)]

mod bindings;
mod catalog;
mod lookup;
mod secret_provider_tests;
mod tags;
pub mod webhooks;

use std::sync::Arc;

use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;
use zeroize::Zeroizing;

pub use bindings::*;
pub use catalog::*;
pub use lookup::*;
pub use secret_provider_tests::*;
pub use tags::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogMutationKind {
    Update,
    Metadata,
    Rename,
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceMetadataError {
    #[error("{0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("credential protection failed")]
    Credential,
    #[error("storage failed: {0}")]
    Storage(String),
}

pub trait ResourceSecretProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<String, ResourceMetadataError>;
    fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, ResourceMetadataError>;
}

pub trait ResourceMetadataStore: Send + Sync {
    fn list_tags<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<TagView>, ResourceMetadataError>>;
    fn create_tag<'a>(
        &'a self,
        actor_id: ActorId,
        tag: &'a NewTag,
    ) -> BoxFuture<'a, Result<TagView, ResourceMetadataError>>;
    fn update_tag<'a>(
        &'a self,
        id: Uuid,
        patch: &'a TagPatch,
    ) -> BoxFuture<'a, Result<TagView, ResourceMetadataError>>;
    fn delete_tag<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;
    fn get_resource_tags<'a>(
        &'a self,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, ResourceMetadataError>>;
    fn replace_resource_tags<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, ResourceMetadataError>>;

    fn list_registries<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<RegistryView>, ResourceMetadataError>>;
    fn get_registry<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>>;
    fn create_registry<'a>(
        &'a self,
        actor_id: ActorId,
        registry: &'a NewRegistry,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>>;
    fn update_registry<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a RegistryPatch,
        kind: CatalogMutationKind,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>>;
    fn delete_registries<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;

    fn list_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryView>, ResourceMetadataError>>;
    fn get_git_repository<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>>;
    fn create_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        repository: &'a NewGitRepository,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>>;
    fn update_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a GitRepositoryPatch,
        kind: CatalogMutationKind,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>>;
    fn delete_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;
    fn update_platform_description<'a>(
        &'a self,
        platform_id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;

    fn get_bindings<'a>(
        &'a self,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>>;
    fn create_binding<'a>(
        &'a self,
        binding: &'a NewResourceBinding,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>>;
    fn update_binding<'a>(
        &'a self,
        binding: &'a ResourceBindingInput,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>>;
    fn delete_binding<'a>(
        &'a self,
        id: Uuid,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>>;
    fn list_secret_definitions<'a>(
        &'a self,
        scope: Option<ResourceBindingScope>,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<Vec<SecretDefinitionView>, ResourceMetadataError>>;
    fn create_internal_secret<'a>(
        &'a self,
        name: &'a str,
        protected_value: &'a str,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>>;
    fn create_external_secret<'a>(
        &'a self,
        input: &'a ExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>>;
    fn update_external_secret<'a>(
        &'a self,
        id: Uuid,
        input: &'a ExternalSecretPatch,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>>;
    fn delete_secret_definition<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;
    fn list_secret_providers<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<SecretProviderView>, ResourceMetadataError>>;
    fn get_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>>;
    fn create_secret_provider<'a>(
        &'a self,
        input: &'a StoredSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>>;
    fn update_secret_provider<'a>(
        &'a self,
        id: Uuid,
        input: &'a StoredSecretProviderPatch,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>>;
    fn delete_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>>;
}

pub struct ResourceMetadataService {
    store: Arc<dyn ResourceMetadataStore>,
    protector: Arc<dyn ResourceSecretProtector>,
    provider_tester: Option<Arc<dyn SecretProviderTester>>,
}

impl ResourceMetadataService {
    #[must_use]
    pub fn new(
        store: Arc<dyn ResourceMetadataStore>,
        protector: Arc<dyn ResourceSecretProtector>,
    ) -> Self {
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
    ) -> Result<SecretTestResult, ResourceMetadataError> {
        input.validate()?;
        self.provider_tester
            .as_ref()
            .ok_or_else(|| {
                ResourceMetadataError::Storage("Secret provider testing is not configured.".into())
            })?
            .test_connection(&input)
            .await
    }

    pub async fn test_external_secret(
        &self,
        input: TestExternalSecretInput,
    ) -> Result<SecretTestResult, ResourceMetadataError> {
        input.validate()?;
        self.provider_tester
            .as_ref()
            .ok_or_else(|| {
                ResourceMetadataError::Storage("Secret provider testing is not configured.".into())
            })?
            .test_external(&input)
            .await
    }

    #[must_use]
    pub fn store(&self) -> &Arc<dyn ResourceMetadataStore> {
        &self.store
    }

    pub async fn create_internal_secret(
        &self,
        name: &str,
        value: &str,
    ) -> Result<SecretDefinitionView, ResourceMetadataError> {
        validate_name(name, "Secret")?;
        if value.is_empty() || value.len() > 256 * 1024 {
            return Err(ResourceMetadataError::Validation(
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
    ) -> Result<SecretProviderView, ResourceMetadataError> {
        validate_provider(&input.name, &input.address, &input.mount_path)?;
        if input.token.is_empty() {
            return Err(ResourceMetadataError::Validation(
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
    ) -> Result<SecretProviderView, ResourceMetadataError> {
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

pub(crate) fn validate_name(name: &str, resource: &str) -> Result<(), ResourceMetadataError> {
    let length = name.trim().chars().count();
    if length == 0 || length > 128 {
        return Err(ResourceMetadataError::Validation(format!(
            "{resource} name must contain between 1 and 128 characters."
        )));
    }
    Ok(())
}
