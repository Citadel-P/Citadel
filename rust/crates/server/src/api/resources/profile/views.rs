use crate::api::resources::common::ResourceCapabilities;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum CurrentProfileAuthenticationType {
    Local,
    Oidc,
}

impl From<CurrentProfileAuthenticationType> for citadel_identity::CurrentProfileAuthenticationType {
    fn from(value: CurrentProfileAuthenticationType) -> Self {
        match value {
            CurrentProfileAuthenticationType::Local => Self::Local,
            CurrentProfileAuthenticationType::Oidc => Self::Oidc,
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthenticationType> for CurrentProfileAuthenticationType {
    fn from(value: citadel_identity::CurrentProfileAuthenticationType) -> Self {
        match value {
            citadel_identity::CurrentProfileAuthenticationType::Local => Self::Local,
            citadel_identity::CurrentProfileAuthenticationType::Oidc => Self::Oidc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResourceInfo {
    pub id: Uuid,
    pub name: String,
}

impl From<ProfileResourceInfo> for citadel_identity::ProfileResourceInfo {
    fn from(value: ProfileResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::ProfileResourceInfo> for ProfileResourceInfo {
    fn from(value: citadel_identity::ProfileResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthenticationView {
    pub r#type: CurrentProfileAuthenticationType,
    pub label: String,
    pub can_change_password: bool,
    pub can_use_local_password_mfa: bool,
    #[schema(required = true)]
    pub oidc_provider_id: Option<Uuid>,
    #[schema(required = true)]
    pub oidc_provider_name: Option<String>,
}

impl From<CurrentProfileAuthenticationView>
    for citadel_identity::CurrentProfileAuthenticationDetails
{
    fn from(value: CurrentProfileAuthenticationView) -> Self {
        Self {
            r#type: value.r#type.into(),
            label: value.label,
            can_change_password: value.can_change_password,
            can_use_local_password_mfa: value.can_use_local_password_mfa,
            oidc_provider_id: value.oidc_provider_id,
            oidc_provider_name: value.oidc_provider_name,
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthenticationDetails>
    for CurrentProfileAuthenticationView
{
    fn from(value: citadel_identity::CurrentProfileAuthenticationDetails) -> Self {
        Self {
            r#type: value.r#type.into(),
            label: value.label,
            can_change_password: value.can_change_password,
            can_use_local_password_mfa: value.can_use_local_password_mfa,
            oidc_provider_id: value.oidc_provider_id,
            oidc_provider_name: value.oidc_provider_name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthorizationView {
    pub is_administrator: bool,
    pub alert_rules: ResourceCapabilities,
    pub bindings: ResourceCapabilities,
    pub tags: ResourceCapabilities,
}

impl From<CurrentProfileAuthorizationView>
    for citadel_identity::CurrentProfileAuthorizationDetails
{
    fn from(value: CurrentProfileAuthorizationView) -> Self {
        Self {
            is_administrator: value.is_administrator,
            alert_rules: value.alert_rules.into(),
            bindings: value.bindings.into(),
            tags: value.tags.into(),
        }
    }
}

impl From<citadel_identity::CurrentProfileAuthorizationDetails>
    for CurrentProfileAuthorizationView
{
    fn from(value: citadel_identity::CurrentProfileAuthorizationDetails) -> Self {
        Self {
            is_administrator: value.is_administrator,
            alert_rules: value.alert_rules.into(),
            bindings: value.bindings.into(),
            tags: value.tags.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileView {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub authentication: CurrentProfileAuthenticationView,
    pub authorization: CurrentProfileAuthorizationView,
    pub created_at: DateTime<Utc>,
    pub direct_roles: Vec<ProfileResourceInfo>,
    pub teams: Vec<ProfileResourceInfo>,
}

impl From<CurrentProfileView> for citadel_identity::CurrentProfileDetails {
    fn from(value: CurrentProfileView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            email: value.email,
            authentication: value.authentication.into(),
            authorization: value.authorization.into(),
            created_at: value.created_at,
            direct_roles: value
                .direct_roles
                .into_iter()
                .map(|item| item.into())
                .collect(),
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<citadel_identity::CurrentProfileDetails> for CurrentProfileView {
    fn from(value: citadel_identity::CurrentProfileDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            email: value.email,
            authentication: value.authentication.into(),
            authorization: value.authorization.into(),
            created_at: value.created_at,
            direct_roles: value
                .direct_roles
                .into_iter()
                .map(|item| item.into())
                .collect(),
            teams: value.teams.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionSummaryView {
    pub id: Uuid,
    pub display_name: String,
    #[schema(required = true)]
    pub user_agent: Option<String>,
    #[schema(required = true)]
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

impl From<UserSessionSummaryView> for citadel_identity::UserSessionSummaryDetails {
    fn from(value: UserSessionSummaryView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            expires_at: value.expires_at,
            is_current: value.is_current,
        }
    }
}

impl From<citadel_identity::UserSessionSummaryDetails> for UserSessionSummaryView {
    fn from(value: citadel_identity::UserSessionSummaryDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            expires_at: value.expires_at,
            is_current: value.is_current,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionsView {
    pub sessions: Vec<UserSessionSummaryView>,
    pub can_revoke_other_sessions: bool,
}

impl From<UserSessionsView> for citadel_identity::UserSessionsDetails {
    fn from(value: UserSessionsView) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(|item| item.into()).collect(),
            can_revoke_other_sessions: value.can_revoke_other_sessions,
        }
    }
}

impl From<citadel_identity::UserSessionsDetails> for UserSessionsView {
    fn from(value: citadel_identity::UserSessionsDetails) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(|item| item.into()).collect(),
            can_revoke_other_sessions: value.can_revoke_other_sessions,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct RevokeOtherProfileSessionsView {
    pub count: i64,
}

impl From<RevokeOtherProfileSessionsView> for citadel_identity::RevokeOtherProfileSessionsDetails {
    fn from(value: RevokeOtherProfileSessionsView) -> Self {
        Self { count: value.count }
    }
}

impl From<citadel_identity::RevokeOtherProfileSessionsDetails> for RevokeOtherProfileSessionsView {
    fn from(value: citadel_identity::RevokeOtherProfileSessionsDetails) -> Self {
        Self { count: value.count }
    }
}
