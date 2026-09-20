use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCitadelRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

impl From<InitializeCitadelRequest> for citadel_identity::InitializeCitadel {
    fn from(value: InitializeCitadelRequest) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
        }
    }
}

impl From<citadel_identity::InitializeCitadel> for InitializeCitadelRequest {
    fn from(value: citadel_identity::InitializeCitadel) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email_or_name: String,
    pub password: String,
}

impl From<LoginRequest> for citadel_identity::Login {
    fn from(value: LoginRequest) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}

impl From<citadel_identity::Login> for LoginRequest {
    fn from(value: citadel_identity::Login) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}
