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
