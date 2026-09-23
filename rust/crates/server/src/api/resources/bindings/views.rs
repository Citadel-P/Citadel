use crate::api::resources::{
    bindings::spec::{
        ResourceBindingKind, ResourceBindingScope, SecretDeliveryMode, SecretProviderType,
    },
    platforms::views::ResourceCapabilitiesView,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GlobalBindingsResponse {
    #[serde(flatten)]
    pub(crate) bindings: ResourceBindingsView,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretDefinitionsResponse {
    pub(crate) secrets: Vec<SecretDefinitionView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SecretProvidersResponse {
    pub(crate) providers: Vec<SecretProviderView>,
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
