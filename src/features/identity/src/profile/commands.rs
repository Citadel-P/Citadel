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
    pub date_display: PatchField<UserDateDisplay>,
    #[serde(default)]
    pub theme: PatchField<UserTheme>,
    #[serde(default)]
    pub theme_color: PatchField<UserThemeColor>,
    #[serde(default)]
    pub font: PatchField<UserUiFont>,
    #[serde(default)]
    pub radius: PatchField<UserUiRadius>,
    #[serde(default)]
    pub content_layout: PatchField<UserContentLayout>,
    #[serde(default)]
    pub density: PatchField<UserUiDensity>,
}

#[derive(Debug, Clone, Default)]
pub struct UserPreferencesUpdate {
    pub time_zone: Option<String>,
    pub date_time_format: Option<UserDateTimeFormat>,
    pub date_display: Option<UserDateDisplay>,
    pub theme: Option<UserTheme>,
    pub theme_color: Option<UserThemeColor>,
    pub font: Option<UserUiFont>,
    pub radius: Option<UserUiRadius>,
    pub content_layout: Option<UserContentLayout>,
    pub density: Option<UserUiDensity>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentPassword {
    pub current_password: String,
    pub new_password: String,
}

impl std::fmt::Debug for ChangeCurrentPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeCurrentPassword")
            .field("current_password", &"[REDACTED]")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}
