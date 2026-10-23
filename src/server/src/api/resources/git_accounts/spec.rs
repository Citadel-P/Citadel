use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum GitTransport {
    Http,
    Https,
    Ssh,
}

impl From<citadel_git::GitTransport> for GitTransport {
    fn from(value: citadel_git::GitTransport) -> Self {
        match value {
            citadel_git::GitTransport::Http => Self::Http,
            citadel_git::GitTransport::Https => Self::Https,
            citadel_git::GitTransport::Ssh => Self::Ssh,
        }
    }
}

impl From<GitTransport> for citadel_git::GitTransport {
    fn from(value: GitTransport) -> Self {
        match value {
            GitTransport::Http => Self::Http,
            GitTransport::Https => Self::Https,
            GitTransport::Ssh => Self::Ssh,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum GitAuthType {
    Basic,
    Token,
    SshKey,
}

impl From<citadel_git::GitAuthType> for GitAuthType {
    fn from(value: citadel_git::GitAuthType) -> Self {
        match value {
            citadel_git::GitAuthType::Basic => Self::Basic,
            citadel_git::GitAuthType::Token => Self::Token,
            citadel_git::GitAuthType::SshKey => Self::SshKey,
        }
    }
}

impl From<GitAuthType> for citadel_git::GitAuthType {
    fn from(value: GitAuthType) -> Self {
        match value {
            GitAuthType::Basic => Self::Basic,
            GitAuthType::Token => Self::Token,
            GitAuthType::SshKey => Self::SshKey,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum GitAuthConfiguration {
    Basic {
        username: String,
        password: String,
    },
    Token {
        token: String,
    },
    SshKey {
        username: String,
        #[serde(rename = "privateKey")]
        private_key: String,
        passphrase: Option<String>,
    },
}

impl From<citadel_git::GitAuthConfiguration> for GitAuthConfiguration {
    fn from(value: citadel_git::GitAuthConfiguration) -> Self {
        match value {
            citadel_git::GitAuthConfiguration::Basic { username, password } => {
                Self::Basic { username, password }
            }
            citadel_git::GitAuthConfiguration::Token { token } => Self::Token { token },
            citadel_git::GitAuthConfiguration::SshKey {
                username,
                private_key,
                passphrase,
            } => Self::SshKey {
                username,
                private_key,
                passphrase,
            },
        }
    }
}

impl From<GitAuthConfiguration> for citadel_git::GitAuthConfiguration {
    fn from(value: GitAuthConfiguration) -> Self {
        match value {
            GitAuthConfiguration::Basic { username, password } => {
                Self::Basic { username, password }
            }
            GitAuthConfiguration::Token { token } => Self::Token { token },
            GitAuthConfiguration::SshKey {
                username,
                private_key,
                passphrase,
            } => Self::SshKey {
                username,
                private_key,
                passphrase,
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(tag = "$type", deny_unknown_fields)]
pub enum GitAuthConfigurationPatch {
    Basic {
        #[serde(default)]
        #[schema(value_type = String, required = false)]
        username: citadel_primitives::FieldUpdate<String>,
        #[serde(default)]
        #[schema(value_type = String, required = false)]
        password: citadel_primitives::FieldUpdate<String>,
    },
    Token {
        #[serde(default)]
        #[schema(value_type = String, required = false)]
        token: citadel_primitives::FieldUpdate<String>,
    },
    SshKey {
        #[serde(default)]
        #[schema(value_type = String, required = false)]
        username: citadel_primitives::FieldUpdate<String>,
        #[serde(rename = "privateKey")]
        #[serde(default)]
        #[schema(value_type = String, required = false)]
        private_key: citadel_primitives::FieldUpdate<String>,
        #[serde(default)]
        #[schema(value_type = Option<String>, required = false)]
        passphrase: citadel_primitives::FieldUpdate<Option<String>>,
    },
}

impl From<GitAuthConfigurationPatch> for citadel_git::GitAuthConfigurationPatch {
    fn from(value: GitAuthConfigurationPatch) -> Self {
        match value {
            GitAuthConfigurationPatch::Basic { username, password } => {
                Self::Basic { username, password }
            }
            GitAuthConfigurationPatch::Token { token } => Self::Token { token },
            GitAuthConfigurationPatch::SshKey {
                username,
                private_key,
                passphrase,
            } => Self::SshKey {
                username,
                private_key,
                passphrase,
            },
        }
    }
}
impl From<citadel_git::GitAuthConfigurationPatch> for GitAuthConfigurationPatch {
    fn from(value: citadel_git::GitAuthConfigurationPatch) -> Self {
        match value {
            citadel_git::GitAuthConfigurationPatch::Basic { username, password } => {
                Self::Basic { username, password }
            }
            citadel_git::GitAuthConfigurationPatch::Token { token } => Self::Token { token },
            citadel_git::GitAuthConfigurationPatch::SshKey {
                username,
                private_key,
                passphrase,
            } => Self::SshKey {
                username,
                private_key,
                passphrase,
            },
        }
    }
}
