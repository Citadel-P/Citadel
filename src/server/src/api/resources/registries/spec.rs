use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum RegistryKind {
    Custom,
    DockerHub,
    Azure,
    AWS,
    Gitlab,
    GitHub,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum RegistrySpec {
    #[serde(rename_all = "camelCase")]
    Custom {
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    DockerHub {
        #[serde(skip_serializing_if = "Option::is_none")]
        user_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pat: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Azure { user_name: String, password: String },
    #[serde(rename_all = "camelCase")]
    AWS {
        access_key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        authentication_required: Option<bool>,
        secret_access_key: String,
        region: String,
    },
    #[serde(rename_all = "camelCase")]
    Gitlab {
        user_name: String,
        pat: String,
        instance_url: String,
    },
    #[serde(rename_all = "camelCase")]
    GitHub {
        #[serde(skip_serializing_if = "Option::is_none")]
        name_space: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ghcr_auth_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pat: Option<String>,
    },
}

/// Public settings and credential presence; secret values are write-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum RegistryConfigurationView {
    #[serde(rename_all = "camelCase")]
    Custom {
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_name: Option<String>,
        has_password: bool,
    },
    #[serde(rename_all = "camelCase")]
    DockerHub {
        #[serde(skip_serializing_if = "Option::is_none")]
        user_name: Option<String>,
        has_pat: bool,
    },
    #[serde(rename_all = "camelCase")]
    Azure {
        user_name: String,
        has_password: bool,
    },
    #[serde(rename_all = "camelCase")]
    AWS {
        access_key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        authentication_required: Option<bool>,
        has_secret_access_key: bool,
        region: String,
    },
    #[serde(rename_all = "camelCase")]
    Gitlab {
        user_name: String,
        has_pat: bool,
        instance_url: String,
    },
    #[serde(rename_all = "camelCase")]
    GitHub {
        #[serde(skip_serializing_if = "Option::is_none")]
        name_space: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ghcr_auth_enabled: Option<bool>,
        has_pat: bool,
    },
}

impl From<RegistrySpec> for RegistryConfigurationView {
    fn from(value: RegistrySpec) -> Self {
        match value {
            RegistrySpec::Custom {
                auth_enabled,
                user_name,
                password,
            } => Self::Custom {
                auth_enabled,
                user_name,
                has_password: password.is_some_and(|v| !v.is_empty()),
            },
            RegistrySpec::DockerHub { user_name, pat } => Self::DockerHub {
                user_name,
                has_pat: pat.is_some_and(|v| !v.is_empty()),
            },
            RegistrySpec::Azure {
                user_name,
                password,
            } => Self::Azure {
                user_name,
                has_password: !password.is_empty(),
            },
            RegistrySpec::AWS {
                access_key,
                authentication_required,
                secret_access_key,
                region,
            } => Self::AWS {
                access_key,
                authentication_required,
                has_secret_access_key: !secret_access_key.is_empty(),
                region,
            },
            RegistrySpec::Gitlab {
                user_name,
                pat,
                instance_url,
            } => Self::Gitlab {
                user_name,
                has_pat: !pat.is_empty(),
                instance_url,
            },
            RegistrySpec::GitHub {
                name_space,
                ghcr_auth_enabled,
                pat,
            } => Self::GitHub {
                name_space,
                ghcr_auth_enabled,
                has_pat: pat.is_some_and(|v| !v.is_empty()),
            },
        }
    }
}
