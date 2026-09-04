use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use zeroize::Zeroizing;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountInput {
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountPatch {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub transport: Option<GitTransport>,
    pub auth_type: Option<GitAuthType>,
    pub configuration: Option<GitAuthConfiguration>,
}

impl GitAccountInput {
    pub fn validate(&mut self) -> Result<(), GitAccountError> {
        validate_name(&self.name)?;
        validate_domain(&self.domain)?;
        validate_configuration(self.transport, self.auth_type, &self.configuration)?;
        self.name = self.name.trim().to_owned();
        self.domain = self.domain.trim().trim_end_matches('/').to_owned();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountView {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitAccountConfigView {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

#[derive(Debug, Clone)]
pub struct StoredGitAccount {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub created_at: DateTime<Utc>,
    pub protected_configuration: Value,
}

impl From<&StoredGitAccount> for GitAccountView {
    fn from(value: &StoredGitAccount) -> Self {
        Self {
            id: value.id,
            created_by_actor_id: value.created_by_actor_id,
            name: value.name.clone(),
            domain: value.domain.clone(),
            transport: value.transport,
            auth_type: value.auth_type,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitAccountError {
    #[error("{0}")]
    Validation(String),
    #[error("Git account was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("Git credential protection failed")]
    Credential,
    #[error("Git account storage failed: {0}")]
    Storage(String),
}

pub trait GitCredentialProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<String, GitAccountError>;
    fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, GitAccountError>;
}

pub trait GitAccountStore: Send + Sync {
    fn list<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<StoredGitAccount>, GitAccountError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn create<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn update<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), GitAccountError>>;
}

pub struct GitAccountService {
    store: Arc<dyn GitAccountStore>,
    protector: Arc<dyn GitCredentialProtector>,
}

impl GitAccountService {
    #[must_use]
    pub fn new(
        store: Arc<dyn GitAccountStore>,
        protector: Arc<dyn GitCredentialProtector>,
    ) -> Self {
        Self { store, protector }
    }

    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
    ) -> Result<Vec<GitAccountView>, GitAccountError> {
        self.store
            .list(actor_id, administrator)
            .await
            .map(|accounts| accounts.iter().map(GitAccountView::from).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<GitAccountView, GitAccountError> {
        let account = self.store.get(id).await?;
        Ok(GitAccountView::from(&account))
    }

    pub async fn get_config(&self, id: Uuid) -> Result<GitAccountConfigView, GitAccountError> {
        let account = self.store.get(id).await?;
        let configuration = self.unprotect(&account.protected_configuration)?;
        Ok(GitAccountConfigView {
            id: account.id,
            name: account.name,
            domain: account.domain,
            transport: account.transport,
            auth_type: account.auth_type,
            configuration,
        })
    }

    pub async fn create(
        &self,
        actor_id: ActorId,
        mut input: GitAccountInput,
    ) -> Result<GitAccountView, GitAccountError> {
        input.validate()?;
        let account = StoredGitAccount {
            id: Uuid::now_v7(),
            created_by_actor_id: actor_id.value(),
            name: input.name,
            domain: input.domain,
            transport: input.transport,
            auth_type: input.auth_type,
            created_at: Utc::now(),
            protected_configuration: self.protect(&input.configuration)?,
        };
        self.store
            .create(&account)
            .await
            .map(|value| GitAccountView::from(&value))
    }

    pub async fn update(
        &self,
        id: Uuid,
        patch: GitAccountPatch,
    ) -> Result<GitAccountView, GitAccountError> {
        let current = self.store.get(id).await?;
        let current_configuration = self.unprotect(&current.protected_configuration)?;
        let mut input = GitAccountInput {
            name: patch.name.unwrap_or_else(|| current.name.clone()),
            domain: patch.domain.unwrap_or_else(|| current.domain.clone()),
            transport: patch.transport.unwrap_or(current.transport),
            auth_type: patch.auth_type.unwrap_or(current.auth_type),
            configuration: patch.configuration.unwrap_or(current_configuration),
        };
        input.validate()?;
        let account = StoredGitAccount {
            id,
            created_by_actor_id: current.created_by_actor_id,
            name: input.name,
            domain: input.domain,
            transport: input.transport,
            auth_type: input.auth_type,
            created_at: current.created_at,
            protected_configuration: self.protect(&input.configuration)?,
        };
        self.store
            .update(&account)
            .await
            .map(|value| GitAccountView::from(&value))
    }

    pub async fn delete(&self, ids: &[Uuid]) -> Result<(), GitAccountError> {
        if ids.is_empty() {
            return Err(GitAccountError::Validation(
                "At least one Git account ID is required.".to_owned(),
            ));
        }
        self.store.delete(ids).await
    }

    fn protect(&self, configuration: &GitAuthConfiguration) -> Result<Value, GitAccountError> {
        let plaintext =
            serde_json::to_vec(configuration).map_err(|_| GitAccountError::Credential)?;
        let envelope = self.protector.protect(&plaintext)?;
        Ok(json!({ "$protected": envelope }))
    }

    fn unprotect(&self, value: &Value) -> Result<GitAuthConfiguration, GitAccountError> {
        let envelope = value
            .get("$protected")
            .and_then(Value::as_str)
            .ok_or(GitAccountError::Credential)?;
        let plaintext = self.protector.unprotect(envelope)?;
        serde_json::from_slice(&plaintext).map_err(|_| GitAccountError::Credential)
    }
}

fn validate_name(name: &str) -> Result<(), GitAccountError> {
    let name = name.trim();
    if !(3..=64).contains(&name.len())
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(GitAccountError::Validation(
            "Git account name must contain 3 to 64 letters, numbers, hyphens, or underscores."
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_domain(domain: &str) -> Result<(), GitAccountError> {
    let domain = domain.trim().trim_end_matches('/');
    let valid = !domain.is_empty()
        && domain.len() <= 253
        && !domain.contains(['/', ':', '@', ' ', '\r', '\n'])
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        });
    if valid {
        Ok(())
    } else {
        Err(GitAccountError::Validation(
            "Please provide a valid host name, for example github.com.".to_owned(),
        ))
    }
}

fn validate_configuration(
    transport: GitTransport,
    auth_type: GitAuthType,
    configuration: &GitAuthConfiguration,
) -> Result<(), GitAccountError> {
    let matches = matches!(
        (auth_type, configuration),
        (GitAuthType::Basic, GitAuthConfiguration::Basic { .. })
            | (GitAuthType::Token, GitAuthConfiguration::Token { .. })
            | (GitAuthType::SshKey, GitAuthConfiguration::SshKey { .. })
    );
    if !matches {
        return Err(GitAccountError::Validation(
            "Git authentication type does not match its configuration.".to_owned(),
        ));
    }
    match (transport, configuration) {
        (GitTransport::Ssh, GitAuthConfiguration::SshKey { .. })
        | (GitTransport::Http | GitTransport::Https, GitAuthConfiguration::Basic { .. })
        | (GitTransport::Http | GitTransport::Https, GitAuthConfiguration::Token { .. }) => {}
        (GitTransport::Ssh, _) => {
            return Err(GitAccountError::Validation(
                "SSH requires SSH key authentication.".to_owned(),
            ));
        }
        (_, GitAuthConfiguration::SshKey { .. }) => {
            return Err(GitAccountError::Validation(
                "SSH authentication cannot be used with HTTP or HTTPS.".to_owned(),
            ));
        }
    }
    let populated = match configuration {
        GitAuthConfiguration::Basic { username, password } => {
            !username.trim().is_empty() && !password.is_empty()
        }
        GitAuthConfiguration::Token { token } => !token.is_empty(),
        GitAuthConfiguration::SshKey {
            username,
            private_key,
            ..
        } => !username.trim().is_empty() && !private_key.trim().is_empty(),
    };
    if populated {
        Ok(())
    } else {
        Err(GitAccountError::Validation(
            "Git credentials cannot be empty.".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use zeroize::Zeroizing;

    #[derive(Default)]
    struct MemoryStore {
        accounts: Mutex<Vec<StoredGitAccount>>,
    }

    impl GitAccountStore for MemoryStore {
        fn list<'a>(
            &'a self,
            _actor_id: ActorId,
            _administrator: bool,
        ) -> BoxFuture<'a, Result<Vec<StoredGitAccount>, GitAccountError>> {
            Box::pin(async { Ok(self.accounts.lock().unwrap().clone()) })
        }

        fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
            Box::pin(async move {
                self.accounts
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|account| account.id == id)
                    .cloned()
                    .ok_or(GitAccountError::NotFound)
            })
        }

        fn create<'a>(
            &'a self,
            account: &'a StoredGitAccount,
        ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
            Box::pin(async move {
                self.accounts.lock().unwrap().push(account.clone());
                Ok(account.clone())
            })
        }

        fn update<'a>(
            &'a self,
            account: &'a StoredGitAccount,
        ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
            Box::pin(async move {
                let mut accounts = self.accounts.lock().unwrap();
                let current = accounts
                    .iter_mut()
                    .find(|current| current.id == account.id)
                    .ok_or(GitAccountError::NotFound)?;
                *current = account.clone();
                Ok(account.clone())
            })
        }

        fn delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), GitAccountError>> {
            Box::pin(async move {
                self.accounts
                    .lock()
                    .unwrap()
                    .retain(|account| !ids.contains(&account.id));
                Ok(())
            })
        }
    }

    struct ReversingProtector;

    impl GitCredentialProtector for ReversingProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<String, GitAccountError> {
            let value = std::str::from_utf8(plaintext).map_err(|_| GitAccountError::Credential)?;
            Ok(value.chars().rev().collect())
        }

        fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, GitAccountError> {
            Ok(Zeroizing::new(
                envelope.chars().rev().collect::<String>().into_bytes(),
            ))
        }
    }

    fn token_input(token: &str) -> GitAccountInput {
        GitAccountInput {
            name: "account-one".to_owned(),
            domain: "git.example.test".to_owned(),
            transport: GitTransport::Https,
            auth_type: GitAuthType::Token,
            configuration: GitAuthConfiguration::Token {
                token: token.to_owned(),
            },
        }
    }

    #[tokio::test]
    async fn credentials_are_protected_before_the_store_receives_them() {
        let store = Arc::new(MemoryStore::default());
        let service = GitAccountService::new(store.clone(), Arc::new(ReversingProtector));
        let actor_id = ActorId::new(Uuid::now_v7());

        let created = service
            .create(actor_id, token_input("secret-token"))
            .await
            .unwrap();

        let persisted = store.accounts.lock().unwrap()[0]
            .protected_configuration
            .to_string();
        assert!(!persisted.contains("secret-token"));
        let config = service.get_config(created.id).await.unwrap();
        assert_eq!(
            config.configuration,
            GitAuthConfiguration::Token {
                token: "secret-token".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn partial_update_preserves_unspecified_credentials_and_settings() {
        let store = Arc::new(MemoryStore::default());
        let service = GitAccountService::new(store, Arc::new(ReversingProtector));
        let actor_id = ActorId::new(Uuid::now_v7());
        let created = service
            .create(actor_id, token_input("secret-token"))
            .await
            .unwrap();

        let updated = service
            .update(
                created.id,
                GitAccountPatch {
                    name: Some("renamed-account".to_owned()),
                    ..GitAccountPatch::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "renamed-account");
        assert_eq!(updated.domain, "git.example.test");
        assert_eq!(updated.transport, GitTransport::Https);
        assert_eq!(updated.auth_type, GitAuthType::Token);
        assert_eq!(
            service.get_config(created.id).await.unwrap().configuration,
            GitAuthConfiguration::Token {
                token: "secret-token".to_owned()
            }
        );
    }

    #[test]
    fn mismatched_authentication_configuration_is_rejected() {
        let mut input = token_input("secret-token");
        input.auth_type = GitAuthType::Basic;

        assert!(matches!(
            input.validate(),
            Err(GitAccountError::Validation(_))
        ));
    }

    #[test]
    fn ssh_credentials_cannot_be_combined_with_https() {
        let mut input = GitAccountInput {
            name: "account-one".to_owned(),
            domain: "git.example.test".to_owned(),
            transport: GitTransport::Https,
            auth_type: GitAuthType::SshKey,
            configuration: GitAuthConfiguration::SshKey {
                username: "git".to_owned(),
                private_key: "private-key".to_owned(),
                passphrase: None,
            },
        };

        assert!(matches!(
            input.validate(),
            Err(GitAccountError::Validation(_))
        ));
    }
}
