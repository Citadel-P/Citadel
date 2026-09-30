use crate::api::resources::bindings::spec::{
    ResourceBindingKind, ResourceBindingScope, SecretDeliveryMode,
};
use citadel_primitives::PatchField;
use citadel_primitives::ResourceType;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InternalSecretInput {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretScopeQuery {
    pub(crate) scope: Option<ResourceBindingScope>,
    pub(crate) resource_id: Option<Uuid>,
    pub(crate) target_resource_type: Option<ResourceType>,
    pub(crate) target_resource_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewResourceBinding {
    pub name: String,
    pub kind: ResourceBindingKind,
    #[serde(skip)]
    pub scope: Option<ResourceBindingScope>,
    #[serde(skip)]
    pub resource_id: Option<Uuid>,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
}

impl From<NewResourceBinding> for citadel_bindings::NewResourceBinding {
    fn from(value: NewResourceBinding) -> Self {
        Self {
            name: value.name,
            kind: value.kind.into(),
            scope: value.scope.map(|item| item.into()),
            resource_id: value.resource_id,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
        }
    }
}

impl From<citadel_bindings::NewResourceBinding> for NewResourceBinding {
    fn from(value: citadel_bindings::NewResourceBinding) -> Self {
        Self {
            name: value.name,
            kind: value.kind.into(),
            scope: value.scope.map(|item| item.into()),
            resource_id: value.resource_id,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingInput {
    pub id: Uuid,
    pub name: String,
    pub kind: ResourceBindingKind,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
}

impl From<ResourceBindingInput> for citadel_bindings::ResourceBindingInput {
    fn from(value: ResourceBindingInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind.into(),
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
        }
    }
}

impl From<citadel_bindings::ResourceBindingInput> for ResourceBindingInput {
    fn from(value: citadel_bindings::ResourceBindingInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind.into(),
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSecretInput {
    pub name: String,
    pub provider_id: Uuid,
    pub external_path: String,
    pub external_key: String,
    pub external_version: Option<i32>,
}

impl From<ExternalSecretInput> for citadel_bindings::ExternalSecretInput {
    fn from(value: ExternalSecretInput) -> Self {
        Self {
            name: value.name,
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}

impl From<citadel_bindings::ExternalSecretInput> for ExternalSecretInput {
    fn from(value: citadel_bindings::ExternalSecretInput) -> Self {
        Self {
            name: value.name,
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSecretPatch {
    pub name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub external_path: Option<String>,
    pub external_key: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub external_version: PatchField<i32>,
}

impl From<ExternalSecretPatch> for citadel_bindings::ExternalSecretPatch {
    fn from(value: ExternalSecretPatch) -> Self {
        Self {
            name: value.name,
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}

impl From<citadel_bindings::ExternalSecretPatch> for ExternalSecretPatch {
    fn from(value: citadel_bindings::ExternalSecretPatch) -> Self {
        Self {
            name: value.name,
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderInput {
    pub name: String,
    pub address: String,
    pub mount_path: String,
    pub token: String,
}

impl From<SecretProviderInput> for citadel_bindings::SecretProviderInput {
    fn from(value: SecretProviderInput) -> Self {
        Self {
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
    }
}

impl From<citadel_bindings::SecretProviderInput> for SecretProviderInput {
    fn from(value: citadel_bindings::SecretProviderInput) -> Self {
        Self {
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub mount_path: Option<String>,
    pub token: Option<String>,
}

impl From<SecretProviderPatch> for citadel_bindings::SecretProviderPatch {
    fn from(value: SecretProviderPatch) -> Self {
        Self {
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
    }
}

impl From<citadel_bindings::SecretProviderPatch> for SecretProviderPatch {
    fn from(value: citadel_bindings::SecretProviderPatch) -> Self {
        Self {
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestSecretProviderInput {
    pub provider_id: Option<Uuid>,
    pub name: Option<String>,
    pub address: String,
    pub mount_path: String,
    pub token: Option<String>,
}

impl From<TestSecretProviderInput> for citadel_bindings::TestSecretProviderInput {
    fn from(value: TestSecretProviderInput) -> Self {
        Self {
            provider_id: value.provider_id,
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
    }
}

impl From<citadel_bindings::TestSecretProviderInput> for TestSecretProviderInput {
    fn from(value: citadel_bindings::TestSecretProviderInput) -> Self {
        Self {
            provider_id: value.provider_id,
            name: value.name,
            address: value.address,
            mount_path: value.mount_path,
            token: value.token,
        }
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

impl From<TestExternalSecretInput> for citadel_bindings::TestExternalSecretInput {
    fn from(value: TestExternalSecretInput) -> Self {
        Self {
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}

impl From<citadel_bindings::TestExternalSecretInput> for TestExternalSecretInput {
    fn from(value: citadel_bindings::TestExternalSecretInput) -> Self {
        Self {
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
        }
    }
}
