use super::spec::*;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountView {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub created_at: DateTime<Utc>,
}

impl From<citadel_git::GitAccount> for GitAccountView {
    fn from(value: citadel_git::GitAccount) -> Self {
        Self {
            id: value.id,
            created_by_actor_id: value.created_by_actor_id,
            name: value.name,
            domain: value.domain,
            transport: value.transport.into(),
            auth_type: value.auth_type.into(),
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountConfigView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

impl From<citadel_git::GitAccountConfiguration> for GitAccountConfigView {
    fn from(value: citadel_git::GitAccountConfiguration) -> Self {
        Self {
            id: value.id,
            name: value.name,
            domain: value.domain,
            transport: value.transport.into(),
            auth_type: value.auth_type.into(),
            configuration: value.configuration.into(),
        }
    }
}
