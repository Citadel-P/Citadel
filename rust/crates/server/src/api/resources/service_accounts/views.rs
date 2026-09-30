use crate::api::resources::{
    capabilities::ResourceCapabilities,
    common::{PagedResult, ResourceInfo},
};
use chrono::{DateTime, Utc};
use citadel_identity::PermissionGrant;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceAccountsResponse {
    pub(crate) paged_result: PagedResult<ServiceAccountView>,
    pub(crate) capabilities: ResourceCapabilities,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceAccountTokensResponse {
    pub(crate) paged_result: PagedResult<ServiceAccountTokenView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceAccountDetailResponse {
    #[serde(flatten)]
    pub(crate) account: ServiceAccountView,
    pub(crate) capabilities: ServiceAccountCapabilities,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceAccountCapabilities {
    pub(crate) can_use: bool,
    pub(crate) can_manage_credentials: bool,
    pub(crate) can_read: bool,
    pub(crate) can_write: bool,
    pub(crate) can_execute: bool,
}

impl From<PermissionGrant> for ServiceAccountCapabilities {
    fn from(permission: PermissionGrant) -> Self {
        Self {
            can_use: permission.has_specific(SpecificPermission::Use),
            can_manage_credentials: permission.has_specific(SpecificPermission::ManageCredentials),
            can_read: permission.level.grants(PermissionLevel::Read),
            can_write: permission.level.grants(PermissionLevel::Write),
            can_execute: permission.level.grants(PermissionLevel::Execute),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountResourceAccess {
    pub id: Option<Uuid>,
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[serde(default)]
    pub resource_name: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<ServiceAccountResourceAccess> for citadel_identity::ServiceAccountResourceAccess {
    fn from(value: ServiceAccountResourceAccess) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::ServiceAccountResourceAccess> for ServiceAccountResourceAccess {
    fn from(value: citadel_identity::ServiceAccountResourceAccess) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountView {
    pub id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub created_by_actor_id: ActorId,
    pub updated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub archived_at_utc: Option<DateTime<Utc>>,
    pub active_token_count: i64,
    #[schema(required = true)]
    pub last_used_at_utc: Option<DateTime<Utc>>,
    pub teams: Vec<ResourceInfo>,
    pub roles: Vec<ResourceInfo>,
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

impl From<ServiceAccountView> for citadel_identity::ServiceAccountDetails {
    fn from(value: ServiceAccountView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            updated_at: value.updated_at,
            archived_at_utc: value.archived_at_utc,
            active_token_count: value.active_token_count,
            last_used_at_utc: value.last_used_at_utc,
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
            roles: value.roles.into_iter().map(|item| item.into()).collect(),
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::ServiceAccountDetails> for ServiceAccountView {
    fn from(value: citadel_identity::ServiceAccountDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            updated_at: value.updated_at,
            archived_at_utc: value.archived_at_utc,
            active_token_count: value.active_token_count,
            last_used_at_utc: value.last_used_at_utc,
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
            roles: value.roles.into_iter().map(|item| item.into()).collect(),
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAsActorUsageView {
    pub id: Uuid,
    pub name: String,
    pub resource_type: ResourceType,
    pub is_active: bool,
}

impl From<RunAsActorUsageView> for citadel_identity::RunAsActorUsageDetails {
    fn from(value: RunAsActorUsageView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            resource_type: value.resource_type,
            is_active: value.is_active,
        }
    }
}

impl From<citadel_identity::RunAsActorUsageDetails> for RunAsActorUsageView {
    fn from(value: citadel_identity::RunAsActorUsageDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            resource_type: value.resource_type,
            is_active: value.is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountTokenView {
    pub id: Uuid,
    pub name: String,
    pub hint: String,
    #[schema(required = true)]
    pub expires_at_utc: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub last_used_at_utc: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub revoked_at_utc: Option<DateTime<Utc>>,
    #[schema(value_type = Option<crate::api::resources::vocabulary::ActorIdSchema>, required = true)]
    pub revoked_by_actor_id: Option<ActorId>,
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub created_by_actor_id: ActorId,
    pub created_by_name: String,
    pub created_at_utc: DateTime<Utc>,
}

impl From<ServiceAccountTokenView> for citadel_identity::ServiceAccountTokenDetails {
    fn from(value: ServiceAccountTokenView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            hint: value.hint,
            expires_at_utc: value.expires_at_utc,
            last_used_at_utc: value.last_used_at_utc,
            revoked_at_utc: value.revoked_at_utc,
            revoked_by_actor_id: value.revoked_by_actor_id,
            created_by_actor_id: value.created_by_actor_id,
            created_by_name: value.created_by_name,
            created_at_utc: value.created_at_utc,
        }
    }
}

impl From<citadel_identity::ServiceAccountTokenDetails> for ServiceAccountTokenView {
    fn from(value: citadel_identity::ServiceAccountTokenDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            hint: value.hint,
            expires_at_utc: value.expires_at_utc,
            last_used_at_utc: value.last_used_at_utc,
            revoked_at_utc: value.revoked_at_utc,
            revoked_by_actor_id: value.revoked_by_actor_id,
            created_by_actor_id: value.created_by_actor_id,
            created_by_name: value.created_by_name,
            created_at_utc: value.created_at_utc,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedServiceAccountTokenView {
    #[serde(flatten)]
    pub credential: ServiceAccountTokenView,
    pub token: String,
}

impl From<CreatedServiceAccountTokenView> for citadel_identity::CreatedServiceAccountTokenDetails {
    fn from(value: CreatedServiceAccountTokenView) -> Self {
        Self {
            credential: value.credential.into(),
            token: value.token,
        }
    }
}

impl From<citadel_identity::CreatedServiceAccountTokenDetails> for CreatedServiceAccountTokenView {
    fn from(value: citadel_identity::CreatedServiceAccountTokenDetails) -> Self {
        Self {
            credential: value.credential.into(),
            token: value.token,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountLimitsView {
    pub default_token_lifetime_days: i64,
    pub maximum_token_lifetime_days: i64,
    pub maximum_active_tokens_per_account: i64,
}

impl From<ServiceAccountLimitsView> for citadel_identity::ServiceAccountLimitsDetails {
    fn from(value: ServiceAccountLimitsView) -> Self {
        Self {
            default_token_lifetime_days: value.default_token_lifetime_days,
            maximum_token_lifetime_days: value.maximum_token_lifetime_days,
            maximum_active_tokens_per_account: value.maximum_active_tokens_per_account,
        }
    }
}

impl From<citadel_identity::ServiceAccountLimitsDetails> for ServiceAccountLimitsView {
    fn from(value: citadel_identity::ServiceAccountLimitsDetails) -> Self {
        Self {
            default_token_lifetime_days: value.default_token_lifetime_days,
            maximum_token_lifetime_days: value.maximum_token_lifetime_days,
            maximum_active_tokens_per_account: value.maximum_active_tokens_per_account,
        }
    }
}
