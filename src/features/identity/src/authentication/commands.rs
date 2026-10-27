use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCitadel {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub update_checks_enabled: Option<bool>,
    #[serde(default)]
    pub mfa_policy: Option<crate::MfaPolicy>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub email_or_name: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub metadata: SessionMetadata,
}

impl std::fmt::Debug for InitializeCitadel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitializeCitadel")
            .field("name", &self.name)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Login")
            .field("email_or_name", &self.email_or_name)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstallationSettings {
    #[serde(default)]
    pub update_checks_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    pub mfa_policy: citadel_primitives::PatchField<crate::MfaPolicy>,
}
