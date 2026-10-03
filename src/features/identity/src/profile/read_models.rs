use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResourceInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthenticationDetails {
    pub r#type: CurrentProfileAuthenticationType,
    pub label: String,
    pub can_change_password: bool,
    pub can_use_local_password_mfa: bool,
    pub oidc_provider_id: Option<Uuid>,
    pub oidc_provider_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileAuthorizationDetails {
    pub is_administrator: bool,
    pub alert_rules: ResourceCapabilities,
    pub bindings: ResourceCapabilities,
    pub tags: ResourceCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentProfileDetails {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub authentication: CurrentProfileAuthenticationDetails,
    pub authorization: CurrentProfileAuthorizationDetails,
    pub created_at: DateTime<Utc>,
    pub direct_roles: Vec<ProfileResourceInfo>,
    pub teams: Vec<ProfileResourceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferencesDetails {
    pub time_zone: Option<String>,
    pub date_time_format: UserDateTimeFormat,
    pub theme: UserTheme,
    pub theme_color: UserThemeColor,
    pub font: UserUiFont,
    pub radius: UserUiRadius,
    pub content_layout: UserContentLayout,
    pub density: UserUiDensity,

    pub is_persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionSummaryDetails {
    pub id: Uuid,
    pub display_name: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionsDetails {
    pub sessions: Vec<UserSessionSummaryDetails>,
    pub can_revoke_other_sessions: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RevokeOtherProfileSessionsDetails {
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct CurrentProfileRecord {
    pub id: Uuid,
    pub actor_id: citadel_primitives::ActorId,
    pub display_name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub has_local_password: bool,
    pub oidc_provider_id: Option<Uuid>,
    pub oidc_provider_name: Option<String>,
    pub direct_roles: Vec<ProfileResourceInfo>,
    pub teams: Vec<ProfileResourceInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CurrentProfileAuthenticationType {
    Local,
    Oidc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordChangeOutcome {
    Changed,
    CurrentPasswordMismatch,
    ExternallyManaged,
}
