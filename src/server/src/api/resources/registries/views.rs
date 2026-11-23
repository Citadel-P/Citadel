use super::spec::{RegistryConfigurationView, RegistryKind};
use crate::api::resources::{capabilities::ResourceCapabilitiesView, tags::views::TagSummary};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistriesResponse {
    pub(crate) registries: Vec<AuthorizedRegistryView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedRegistryView {
    #[serde(flatten)]
    pub(crate) registry: RegistryView,
    pub(crate) capabilities: ResourceCapabilitiesView,
    pub(crate) is_default: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryConfigResponse {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) registry_host: String,
    #[schema(value_type = crate::api::resources::schema_models::registries::RegistryStatusSchema)]
    pub(crate) status: crate::api::resources::registries::views::RegistryStatus,
    pub(crate) description: String,
    pub(crate) configuration: RegistryConfigurationView,
    pub(crate) tags: Vec<crate::api::resources::tags::views::TagSummary>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

pub use citadel_registries::RegistryStatus;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryView {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::resources::schema_models::registries::RegistryStatusSchema)]
    pub status: RegistryStatus,
    #[schema(required = true)]
    pub description: Option<String>,
    pub registry_host: String,
    #[serde(rename = "type")]
    pub registry_type: RegistryKind,
    #[serde(skip_serializing)]
    pub configuration: Value,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<TagSummary>,
}

impl TryFrom<citadel_registries::RegistryDetails> for RegistryView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_registries::RegistryDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            name: value.name,
            status: value.status,
            description: value.description,
            registry_host: value.registry_host,
            registry_type: serde_json::from_value(value.registry_type.into())?,
            configuration: value.configuration,
            created_at: value.audit.created_at,
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
        })
    }
}

pub(crate) async fn authorized_registries(
    identity: &citadel_identity::IdentityService,
    principal: &citadel_identity::ActorPrincipal,
    registries: Vec<citadel_registries::RegistryDetails>,
) -> Result<Vec<AuthorizedRegistryView>, crate::api::error::ApiError> {
    let ids = registries
        .iter()
        .map(|resource| resource.id)
        .collect::<Vec<_>>();
    let permissions = identity
        .permissions_for_resources(principal, citadel_primitives::ResourceType::Registry, &ids)
        .await?;
    registries
        .into_iter()
        .map(|registry| {
            Ok(AuthorizedRegistryView {
                is_default: registry.id == Uuid::from_u128(0x100),
                capabilities: crate::api::resource_access::capabilities_from_permission(
                    permissions.get(&registry.id).copied().flatten(),
                ),
                registry: RegistryView::try_from(registry)
                    .map_err(crate::api::error::ApiError::internal)?,
            })
        })
        .collect()
}
