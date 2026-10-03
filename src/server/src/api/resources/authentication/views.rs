use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccessTokenResponse {
    pub(crate) access_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum LoginNextStep {
    Completed,
    VerifyMfa,
    EnrollMfa,
}

impl From<LoginNextStep> for citadel_identity::LoginNextStep {
    fn from(value: LoginNextStep) -> Self {
        match value {
            LoginNextStep::Completed => Self::Completed,
            LoginNextStep::VerifyMfa => Self::VerifyMfa,
            LoginNextStep::EnrollMfa => Self::EnrollMfa,
        }
    }
}

impl From<citadel_identity::LoginNextStep> for LoginNextStep {
    fn from(value: citadel_identity::LoginNextStep) -> Self {
        match value {
            citadel_identity::LoginNextStep::Completed => Self::Completed,
            citadel_identity::LoginNextStep::VerifyMfa => Self::VerifyMfa,
            citadel_identity::LoginNextStep::EnrollMfa => Self::EnrollMfa,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    #[schema(required = true)]
    pub access_token: Option<String>,
    pub next_step: LoginNextStep,
}

impl From<LoginResponse> for citadel_identity::LoginOutcome {
    fn from(value: LoginResponse) -> Self {
        Self {
            access_token: value.access_token,
            next_step: value.next_step.into(),
        }
    }
}

impl From<citadel_identity::LoginOutcome> for LoginResponse {
    fn from(value: citadel_identity::LoginOutcome) -> Self {
        Self {
            access_token: value.access_token,
            next_step: value.next_step.into(),
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusView {
    pub requires_setup: bool,
    pub password_minimum_length: usize,
    pub password_maximum_length: usize,
}

impl From<citadel_identity::SetupStatus> for SetupStatusView {
    fn from(value: citadel_identity::SetupStatus) -> Self {
        Self {
            requires_setup: value.requires_setup,
            password_minimum_length: value.password_minimum_length,
            password_maximum_length: value.password_maximum_length,
        }
    }
}
