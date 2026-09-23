use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentProfile {
    pub display_name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserPreferences {
    #[serde(default)]
    pub time_zone: PatchField<String>,
    #[serde(default)]
    pub date_time_format: PatchField<UserDateTimeFormat>,
    #[serde(default)]
    pub theme: PatchField<UserTheme>,
}

#[derive(Debug, Clone, Default)]
pub struct UserPreferencesUpdate {
    pub time_zone: Option<String>,
    pub date_time_format: Option<UserDateTimeFormat>,
    pub theme: Option<UserTheme>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentPassword {
    pub current_password: String,
    pub new_password: String,
}
