use crate::api::metadata_patch::MetadataPatch;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceBindingKind {
    Variable,
    Secret,
}

impl From<ResourceBindingKind> for citadel_bindings::ResourceBindingKind {
    fn from(value: ResourceBindingKind) -> Self {
        match value {
            ResourceBindingKind::Variable => Self::Variable,
            ResourceBindingKind::Secret => Self::Secret,
        }
    }
}

impl From<citadel_bindings::ResourceBindingKind> for ResourceBindingKind {
    fn from(value: citadel_bindings::ResourceBindingKind) -> Self {
        match value {
            citadel_bindings::ResourceBindingKind::Variable => Self::Variable,
            citadel_bindings::ResourceBindingKind::Secret => Self::Secret,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceBindingScope {
    Global,
    Stack,
    Deployment,
    SwarmService,
}

impl From<ResourceBindingScope> for citadel_bindings::ResourceBindingScope {
    fn from(value: ResourceBindingScope) -> Self {
        match value {
            ResourceBindingScope::Global => Self::Global,
            ResourceBindingScope::Stack => Self::Stack,
            ResourceBindingScope::Deployment => Self::Deployment,
            ResourceBindingScope::SwarmService => Self::SwarmService,
        }
    }
}

impl From<citadel_bindings::ResourceBindingScope> for ResourceBindingScope {
    fn from(value: citadel_bindings::ResourceBindingScope) -> Self {
        match value {
            citadel_bindings::ResourceBindingScope::Global => Self::Global,
            citadel_bindings::ResourceBindingScope::Stack => Self::Stack,
            citadel_bindings::ResourceBindingScope::Deployment => Self::Deployment,
            citadel_bindings::ResourceBindingScope::SwarmService => Self::SwarmService,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SecretDeliveryMode {
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret,
}

impl From<SecretDeliveryMode> for citadel_bindings::SecretDeliveryMode {
    fn from(value: SecretDeliveryMode) -> Self {
        match value {
            SecretDeliveryMode::EnvironmentVariable => Self::EnvironmentVariable,
            SecretDeliveryMode::MountedFile => Self::MountedFile,
            SecretDeliveryMode::NativePlatformSecret => Self::NativePlatformSecret,
        }
    }
}

impl From<citadel_bindings::SecretDeliveryMode> for SecretDeliveryMode {
    fn from(value: citadel_bindings::SecretDeliveryMode) -> Self {
        match value {
            citadel_bindings::SecretDeliveryMode::EnvironmentVariable => Self::EnvironmentVariable,
            citadel_bindings::SecretDeliveryMode::MountedFile => Self::MountedFile,
            citadel_bindings::SecretDeliveryMode::NativePlatformSecret => {
                Self::NativePlatformSecret
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SecretProviderType {
    InternalEncrypted,
    VaultCompatibleKvV2,
}

impl From<SecretProviderType> for citadel_bindings::SecretProviderType {
    fn from(value: SecretProviderType) -> Self {
        match value {
            SecretProviderType::InternalEncrypted => Self::InternalEncrypted,
            SecretProviderType::VaultCompatibleKvV2 => Self::VaultCompatibleKvV2,
        }
    }
}

impl From<citadel_bindings::SecretProviderType> for SecretProviderType {
    fn from(value: citadel_bindings::SecretProviderType) -> Self {
        match value {
            citadel_bindings::SecretProviderType::InternalEncrypted => Self::InternalEncrypted,
            citadel_bindings::SecretProviderType::VaultCompatibleKvV2 => Self::VaultCompatibleKvV2,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingView {
    pub id: Uuid,
    pub name: String,
    pub kind: ResourceBindingKind,
    pub scope: ResourceBindingScope,
    pub resource_id: Option<Uuid>,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
    pub is_inherited: bool,
}

impl From<ResourceBindingView> for citadel_bindings::ResourceBinding {
    fn from(value: ResourceBindingView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind.into(),
            scope: value.scope.into(),
            resource_id: value.resource_id,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
            is_inherited: value.is_inherited,
        }
    }
}

impl From<citadel_bindings::ResourceBinding> for ResourceBindingView {
    fn from(value: citadel_bindings::ResourceBinding) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind.into(),
            scope: value.scope.into(),
            resource_id: value.resource_id,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value.secret_delivery_mode.map(|item| item.into()),
            target_path: value.target_path,
            is_inherited: value.is_inherited,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingsView {
    pub entries: Vec<ResourceBindingView>,
    pub effective_entries: Vec<ResourceBindingView>,
}

impl From<ResourceBindingsView> for citadel_bindings::ResourceBindings {
    fn from(value: ResourceBindingsView) -> Self {
        Self {
            entries: value.entries.into_iter().map(|item| item.into()).collect(),
            effective_entries: value
                .effective_entries
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_bindings::ResourceBindings> for ResourceBindingsView {
    fn from(value: citadel_bindings::ResourceBindings) -> Self {
        Self {
            entries: value.entries.into_iter().map(|item| item.into()).collect(),
            effective_entries: value
                .effective_entries
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
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

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretDefinitionView {
    pub id: Uuid,
    pub name: String,
    pub provider_type: SecretProviderType,
    pub provider_id: Option<Uuid>,
    pub external_path: Option<String>,
    pub external_key: Option<String>,
    pub external_version: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl From<SecretDefinitionView> for citadel_bindings::SecretDefinition {
    fn from(value: SecretDefinitionView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_type: value.provider_type.into(),
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
            created_at: value.created_at,
        }
    }
}

impl From<citadel_bindings::SecretDefinition> for SecretDefinitionView {
    fn from(value: citadel_bindings::SecretDefinition) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_type: value.provider_type.into(),
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version,
            created_at: value.created_at,
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
    pub external_version: MetadataPatch<i32>,
}

impl From<ExternalSecretPatch> for citadel_bindings::ExternalSecretPatch {
    fn from(value: ExternalSecretPatch) -> Self {
        Self {
            name: value.name,
            provider_id: value.provider_id,
            external_path: value.external_path,
            external_key: value.external_key,
            external_version: value.external_version.into(),
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
            external_version: value.external_version.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderView {
    pub id: Uuid,
    pub name: String,
    pub provider_type: SecretProviderType,
    pub address: String,
    pub mount_path: String,
    pub created_at: DateTime<Utc>,
}

impl From<SecretProviderView> for citadel_bindings::SecretProvider {
    fn from(value: SecretProviderView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_type: value.provider_type.into(),
            address: value.address,
            mount_path: value.mount_path,
            created_at: value.created_at,
        }
    }
}

impl From<citadel_bindings::SecretProvider> for SecretProviderView {
    fn from(value: citadel_bindings::SecretProvider) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_type: value.provider_type.into(),
            address: value.address,
            mount_path: value.mount_path,
            created_at: value.created_at,
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
