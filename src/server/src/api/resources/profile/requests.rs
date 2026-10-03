use serde::Deserialize;
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentProfileRequest {
    pub display_name: String,
}

impl From<UpdateCurrentProfileRequest> for citadel_identity::UpdateCurrentProfile {
    fn from(value: UpdateCurrentProfileRequest) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

impl From<citadel_identity::UpdateCurrentProfile> for UpdateCurrentProfileRequest {
    fn from(value: citadel_identity::UpdateCurrentProfile) -> Self {
        Self {
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

impl From<ChangeCurrentPasswordRequest> for citadel_identity::ChangeCurrentPassword {
    fn from(value: ChangeCurrentPasswordRequest) -> Self {
        Self {
            current_password: value.current_password,
            new_password: value.new_password,
        }
    }
}

impl From<citadel_identity::ChangeCurrentPassword> for ChangeCurrentPasswordRequest {
    fn from(value: citadel_identity::ChangeCurrentPassword) -> Self {
        Self {
            current_password: value.current_password,
            new_password: value.new_password,
        }
    }
}
