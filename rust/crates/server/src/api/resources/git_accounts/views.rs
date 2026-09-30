use crate::api::resources::{capabilities::ResourceCapabilitiesView, git_accounts::spec::*};
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
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            name: value.name,
            domain: value.domain,
            transport: value.transport.into(),
            auth_type: value.auth_type.into(),
            created_at: value.audit.created_at,
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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitAccountsResponse {
    pub(crate) git_accounts: Vec<AuthorizedGitAccountView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedGitAccountView {
    #[serde(flatten)]
    pub(crate) account: GitAccountView,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_attribution_remains_flat_on_the_wire() {
        let actor = Uuid::now_v7();
        let created_at = Utc::now();
        let view = GitAccountView::from(citadel_git::GitAccount {
            id: Uuid::now_v7(),
            name: "git".into(),
            domain: "git.example.test".into(),
            transport: citadel_git::GitTransport::Https,
            auth_type: citadel_git::GitAuthType::Token,
            audit: citadel_primitives::AuditMetadata {
                created_by_actor_id: citadel_primitives::ActorId::new(actor),
                created_at,
            },
        });
        let wire = serde_json::to_value(view).unwrap();
        assert_eq!(wire["createdByActorId"], serde_json::json!(actor));
        assert_eq!(wire["createdAt"], serde_json::json!(created_at));
        assert!(wire.get("audit").is_none());
    }
}
