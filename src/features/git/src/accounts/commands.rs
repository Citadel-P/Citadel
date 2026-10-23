use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGitAccount {
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGitAccount {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub transport: Option<GitTransport>,
    pub auth_type: Option<GitAuthType>,
    pub configuration: Option<GitAuthConfigurationPatch>,
}

impl CreateGitAccount {
    pub fn validate(&mut self) -> Result<(), GitAccountError> {
        validate_name(&self.name)?;
        validate_domain(&self.domain)?;
        validate_configuration(self.transport, self.auth_type, &self.configuration)?;
        self.name = self.name.trim().to_owned();
        self.domain = self.domain.trim().trim_end_matches('/').to_owned();
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type", deny_unknown_fields)]
pub enum GitAuthConfigurationPatch {
    Basic {
        #[serde(default)]
        username: citadel_primitives::FieldUpdate<String>,
        #[serde(default)]
        password: citadel_primitives::FieldUpdate<String>,
    },
    Token {
        #[serde(default)]
        token: citadel_primitives::FieldUpdate<String>,
    },
    SshKey {
        #[serde(default)]
        username: citadel_primitives::FieldUpdate<String>,
        #[serde(rename = "privateKey")]
        #[serde(default)]
        private_key: citadel_primitives::FieldUpdate<String>,
        #[serde(default)]
        passphrase: citadel_primitives::FieldUpdate<Option<String>>,
    },
}
impl GitAuthConfigurationPatch {
    pub fn apply(self, current: GitAuthConfiguration) -> GitAuthConfiguration {
        match self {
            Self::Basic { username, password } => {
                let (old_user, old_password) = match current {
                    GitAuthConfiguration::Basic { username, password } => (username, password),
                    _ => Default::default(),
                };
                GitAuthConfiguration::Basic {
                    username: username.apply(old_user),
                    password: password.apply(old_password),
                }
            }
            Self::Token { token } => GitAuthConfiguration::Token {
                token: token.apply(match current {
                    GitAuthConfiguration::Token { token } => token,
                    _ => String::new(),
                }),
            },
            Self::SshKey {
                username,
                private_key,
                passphrase,
            } => {
                let (old_user, old_key, old_passphrase) = match current {
                    GitAuthConfiguration::SshKey {
                        username,
                        private_key,
                        passphrase,
                    } => (username, private_key, passphrase),
                    _ => Default::default(),
                };
                GitAuthConfiguration::SshKey {
                    username: username.apply(old_user),
                    private_key: private_key.apply(old_key),
                    passphrase: passphrase.apply(old_passphrase),
                }
            }
        }
    }
}
