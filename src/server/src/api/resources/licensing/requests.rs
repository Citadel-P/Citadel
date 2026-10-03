use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallLicenseRequest {
    pub license: String,
}
