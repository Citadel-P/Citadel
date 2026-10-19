use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitTransport {
    Http,
    Https,
    Ssh,
}

impl GitTransport {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Http => "Http",
            Self::Https => "Https",
            Self::Ssh => "Ssh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitAuthType {
    Basic,
    Token,
    SshKey,
}

impl GitAuthType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Token => "Token",
            Self::SshKey => "SshKey",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct GitAccount {
    pub id: Uuid,
    pub audit: citadel_primitives::AuditMetadata,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
}

#[derive(Debug, Clone)]
pub struct StoredGitAccount {
    pub id: Uuid,
    pub audit: citadel_primitives::AuditMetadata,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub protected_configuration: Value,
}

impl From<&StoredGitAccount> for GitAccount {
    fn from(value: &StoredGitAccount) -> Self {
        Self {
            id: value.id,
            audit: value.audit,
            name: value.name.clone(),
            domain: value.domain.clone(),
            transport: value.transport,
            auth_type: value.auth_type,
        }
    }
}

impl std::fmt::Debug for GitAuthConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("password", &"[REDACTED]")
                .finish(),
            Self::Token { .. } => f
                .debug_struct("Token")
                .field("token", &"[REDACTED]")
                .finish(),
            Self::SshKey { username, .. } => f
                .debug_struct("SshKey")
                .field("username", username)
                .field("private_key", &"[REDACTED]")
                .field("passphrase", &"[REDACTED]")
                .finish(),
        }
    }
}

#[cfg(test)]
mod credential_debug_tests {
    use super::*;

    #[test]
    fn plaintext_credentials_are_redacted_without_changing_serialization() {
        for value in [
            GitAuthConfiguration::Basic {
                username: "user".into(),
                password: "SECRET_PASSWORD".into(),
            },
            GitAuthConfiguration::Token {
                token: "SECRET_TOKEN".into(),
            },
            GitAuthConfiguration::SshKey {
                username: "git".into(),
                private_key: "SECRET_KEY".into(),
                passphrase: Some("SECRET_PASSPHRASE".into()),
            },
        ] {
            assert!(!format!("{value:?}").contains("SECRET_"));
            assert!(serde_json::to_string(&value).unwrap().contains("SECRET_"));
        }
    }
}
