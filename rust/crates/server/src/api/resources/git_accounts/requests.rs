use crate::api::resources::git_accounts::spec::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountInput {
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

impl From<citadel_git::CreateGitAccount> for GitAccountInput {
    fn from(value: citadel_git::CreateGitAccount) -> Self {
        Self {
            name: value.name,
            domain: value.domain,
            transport: value.transport.into(),
            auth_type: value.auth_type.into(),
            configuration: value.configuration.into(),
        }
    }
}

impl From<GitAccountInput> for citadel_git::CreateGitAccount {
    fn from(value: GitAccountInput) -> Self {
        Self {
            name: value.name,
            domain: value.domain,
            transport: value.transport.into(),
            auth_type: value.auth_type.into(),
            configuration: value.configuration.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountPatch {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub transport: Option<GitTransport>,
    pub auth_type: Option<GitAuthType>,
    pub configuration: Option<GitAuthConfiguration>,
}

impl From<citadel_git::UpdateGitAccount> for GitAccountPatch {
    fn from(value: citadel_git::UpdateGitAccount) -> Self {
        Self {
            name: value.name,
            domain: value.domain,
            transport: value.transport.map(Into::into),
            auth_type: value.auth_type.map(Into::into),
            configuration: value.configuration.map(Into::into),
        }
    }
}

impl From<GitAccountPatch> for citadel_git::UpdateGitAccount {
    fn from(value: GitAccountPatch) -> Self {
        Self {
            name: value.name,
            domain: value.domain,
            transport: value.transport.map(Into::into),
            auth_type: value.auth_type.map(Into::into),
            configuration: value.configuration.map(Into::into),
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteGitAccountsInput {
    pub(crate) ids: Vec<Uuid>,
}
