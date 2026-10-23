use super::*;

#[derive(Debug, Clone)]
pub struct GitAccountConfiguration {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthMetadata,
}

/// Nonsecret configuration metadata derived from the saved credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAuthMetadata {
    pub username: Option<String>,
    pub has_password: bool,
    pub has_token: bool,
    pub has_private_key: bool,
    pub has_passphrase: Option<bool>,
}
impl GitAuthMetadata {
    pub fn from_configuration(value: &GitAuthConfiguration) -> Self {
        match value {
            GitAuthConfiguration::Basic { username, password } => Self {
                username: Some(username.clone()),
                has_password: !password.is_empty(),
                ..Self::default()
            },
            GitAuthConfiguration::Token { token } => Self {
                has_token: !token.is_empty(),
                ..Self::default()
            },
            GitAuthConfiguration::SshKey {
                username,
                private_key,
                passphrase,
            } => Self {
                username: Some(username.clone()),
                has_private_key: !private_key.is_empty(),
                has_passphrase: Some(passphrase.is_some()),
                ..Self::default()
            },
        }
    }
}

pub(crate) struct GitAccountCredentials {
    pub domain: String,
    pub transport: GitTransport,
    pub configuration: GitAuthConfiguration,
}
