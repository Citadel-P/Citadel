use crate::api::resources::capabilities::ResourceCapabilitiesView;
use citadel_identity::RoleType;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RolesResponse {
    pub(crate) roles: Vec<RoleView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionMatrixViewItem {
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub(crate) maximum_level: PermissionLevel,
    #[schema(value_type = BTreeMap<String, crate::api::resources::vocabulary::PermissionLevelSchema>)]
    pub(crate) specific_permissions: BTreeMap<String, PermissionLevel>,
    pub(crate) label: String,
    pub(crate) specific_permission_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionView {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<RolePermissionView> for citadel_identity::RolePermissionDetails {
    fn from(value: RolePermissionView) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::RolePermissionDetails> for RolePermissionView {
    fn from(value: citadel_identity::RolePermissionDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleView {
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::resources::vocabulary::RoleTypeSchema)]
    pub role_type: RoleType,
    pub permissions: Vec<RolePermissionView>,
}

impl From<RoleView> for citadel_identity::RoleDetails {
    fn from(value: RoleView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            role_type: value.role_type,
            permissions: value
                .permissions
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::RoleDetails> for RoleView {
    fn from(value: citadel_identity::RoleDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            role_type: value.role_type,
            permissions: value
                .permissions
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}
