use crate::*;
use futures_util::future::BoxFuture;
use uuid::Uuid;
use zeroize::Zeroizing;
pub trait SecretProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<String, BindingError>;
    fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, BindingError>;
}
pub trait BindingRepository: Send + Sync {
    fn get_bindings<'a>(
        &'a self,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>>;

    fn create_binding<'a>(
        &'a self,
        binding: &'a NewResourceBinding,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>>;

    fn update_binding<'a>(
        &'a self,
        binding: &'a ResourceBindingInput,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>>;

    fn delete_binding<'a>(
        &'a self,
        id: Uuid,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>>;

    fn list_secret_definitions<'a>(
        &'a self,
        scope: Option<ResourceBindingScope>,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<Vec<SecretDefinition>, BindingError>>;

    fn create_internal_secret<'a>(
        &'a self,
        name: &'a str,
        protected_value: &'a str,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>>;

    fn create_external_secret<'a>(
        &'a self,
        input: &'a ExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>>;

    fn update_external_secret<'a>(
        &'a self,
        id: Uuid,
        input: &'a ExternalSecretPatch,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>>;

    fn delete_secret_definition<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BindingError>>;

    fn list_secret_providers<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<SecretProvider>, BindingError>>;

    fn get_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>>;

    fn create_secret_provider<'a>(
        &'a self,
        input: &'a StoredSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>>;

    fn update_secret_provider<'a>(
        &'a self,
        id: Uuid,
        input: &'a StoredSecretProviderPatch,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>>;

    fn delete_secret_provider<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BindingError>>;
}
