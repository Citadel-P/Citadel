use crate::api::resources::{
    common::enabled_by_default, service_accounts::views::ServiceAccountResourceAccess,
};
use chrono::{DateTime, Utc};
use citadel_identity::PatchField;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    pub(crate) page: i64,
    #[serde(alias = "PageSize")]
    #[serde(default = "default_page_size")]
    pub(crate) page_size: i64,
    #[serde(alias = "Name")]
    pub(crate) name: Option<String>,
    #[serde(alias = "IncludeArchived")]
    #[serde(default)]
    pub(crate) include_archived: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TokenFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    pub(crate) page: i64,
    #[serde(alias = "PageSize")]
    #[serde(default = "default_page_size")]
    pub(crate) page_size: i64,
}

const fn default_page_size() -> i64 {
    50
}

const fn first_page() -> i64 {
    1
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

impl From<CreateServiceAccountRequest> for citadel_identity::CreateServiceAccount {
    fn from(value: CreateServiceAccountRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateServiceAccount> for CreateServiceAccountRequest {
    fn from(value: citadel_identity::CreateServiceAccount) -> Self {
        Self {
            name: value.name,
            description: value.description,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceAccountRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
}

impl From<UpdateServiceAccountRequest> for citadel_identity::UpdateServiceAccount {
    fn from(value: UpdateServiceAccountRequest) -> Self {
        Self {
            description: value.description,
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::UpdateServiceAccount> for UpdateServiceAccountRequest {
    fn from(value: citadel_identity::UpdateServiceAccount) -> Self {
        Self {
            description: value.description,
            is_enabled: value.is_enabled,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameServiceAccountRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameServiceAccountRequest> for citadel_identity::RenameServiceAccount {
    fn from(value: RenameServiceAccountRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameServiceAccount> for RenameServiceAccountRequest {
    fn from(value: citadel_identity::RenameServiceAccount) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountRoleRequest {
    pub role_id: Uuid,
}

impl From<AddServiceAccountRoleRequest> for citadel_identity::AddServiceAccountRole {
    fn from(value: AddServiceAccountRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddServiceAccountRole> for AddServiceAccountRoleRequest {
    fn from(value: citadel_identity::AddServiceAccountRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountResourceAccessRequest {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<AddServiceAccountResourceAccessRequest>
    for citadel_identity::AddServiceAccountResourceAccess
{
    fn from(value: AddServiceAccountResourceAccessRequest) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::AddServiceAccountResourceAccess>
    for AddServiceAccountResourceAccessRequest
{
    fn from(value: citadel_identity::AddServiceAccountResourceAccess) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveServiceAccountsRequest {
    pub ids: Vec<Uuid>,
}

impl From<ArchiveServiceAccountsRequest> for citadel_identity::ArchiveServiceAccounts {
    fn from(value: ArchiveServiceAccountsRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::ArchiveServiceAccounts> for ArchiveServiceAccountsRequest {
    fn from(value: citadel_identity::ArchiveServiceAccounts) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountTokenRequest {
    pub name: String,
    pub expires_at_utc: Option<DateTime<Utc>>,
    #[serde(default)]
    pub never_expires: bool,
}

impl From<CreateServiceAccountTokenRequest> for citadel_identity::CreateServiceAccountToken {
    fn from(value: CreateServiceAccountTokenRequest) -> Self {
        Self {
            name: value.name,
            expires_at_utc: value.expires_at_utc,
            never_expires: value.never_expires,
        }
    }
}

impl From<citadel_identity::CreateServiceAccountToken> for CreateServiceAccountTokenRequest {
    fn from(value: citadel_identity::CreateServiceAccountToken) -> Self {
        Self {
            name: value.name,
            expires_at_utc: value.expires_at_utc,
            never_expires: value.never_expires,
        }
    }
}
