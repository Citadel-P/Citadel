use super::*;

pub struct GitAccountService {
    store: Arc<dyn GitAccountRepository>,
    protector: Arc<dyn GitCredentialProtector>,
}

impl GitAccountService {
    #[must_use]
    pub fn new(
        store: Arc<dyn GitAccountRepository>,
        protector: Arc<dyn GitCredentialProtector>,
    ) -> Self {
        Self { store, protector }
    }

    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
    ) -> Result<Vec<GitAccount>, GitAccountError> {
        self.store
            .list(actor_id, administrator)
            .await
            .map(|accounts| accounts.iter().map(GitAccount::from).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<GitAccount, GitAccountError> {
        let account = self.store.get(id).await?;
        Ok(GitAccount::from(&account))
    }

    pub async fn get_config(&self, id: Uuid) -> Result<GitAccountConfiguration, GitAccountError> {
        let account = self.store.get(id).await?;
        let configuration =
            GitAuthMetadata::from_configuration(&self.unprotect(&account.protected_configuration)?);
        Ok(GitAccountConfiguration {
            id: account.id,
            name: account.name,
            domain: account.domain,
            transport: account.transport,
            auth_type: account.auth_type,
            configuration,
        })
    }

    pub(crate) async fn credentials(
        &self,
        id: Uuid,
    ) -> Result<super::read_models::GitAccountCredentials, GitAccountError> {
        let account = self.store.get(id).await?;
        Ok(super::read_models::GitAccountCredentials {
            domain: account.domain,
            transport: account.transport,
            configuration: self.unprotect(&account.protected_configuration)?,
        })
    }

    pub async fn create(
        &self,
        actor_id: ActorId,
        mut input: CreateGitAccount,
    ) -> Result<GitAccount, GitAccountError> {
        input.validate()?;
        let account = StoredGitAccount {
            id: Uuid::now_v7(),
            audit: citadel_primitives::AuditMetadata {
                created_by_actor_id: actor_id,
                created_at: Utc::now(),
            },
            name: input.name,
            domain: input.domain,
            transport: input.transport,
            auth_type: input.auth_type,
            protected_configuration: self.protect(&input.configuration)?,
        };
        self.store
            .create(&account)
            .await
            .map(|value| GitAccount::from(&value))
    }

    pub async fn update(
        &self,
        id: Uuid,
        patch: UpdateGitAccount,
    ) -> Result<GitAccount, GitAccountError> {
        let current = self.store.get(id).await?;
        let current_configuration = self.unprotect(&current.protected_configuration)?;
        let mut input = CreateGitAccount {
            name: patch.name.unwrap_or_else(|| current.name.clone()),
            domain: patch.domain.unwrap_or_else(|| current.domain.clone()),
            transport: patch.transport.unwrap_or(current.transport),
            auth_type: patch.auth_type.unwrap_or(current.auth_type),
            configuration: patch.configuration.map_or_else(
                || current_configuration.clone(),
                |patch| patch.apply(current_configuration.clone()),
            ),
        };
        input.validate()?;
        let account = StoredGitAccount {
            id,
            audit: current.audit,
            name: input.name,
            domain: input.domain,
            transport: input.transport,
            auth_type: input.auth_type,
            protected_configuration: self.protect(&input.configuration)?,
        };
        self.store
            .update(&account)
            .await
            .map(|value| GitAccount::from(&value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use zeroize::Zeroizing;

    #[derive(Default)]
    struct MemoryStore {
        accounts: Mutex<Vec<StoredGitAccount>>,
    }

    impl GitAccountRepository for MemoryStore {
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

    fn token_input(token: &str) -> CreateGitAccount {
        CreateGitAccount {
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
            GitAuthMetadata {
                has_token: true,
                ..GitAuthMetadata::default()
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
                UpdateGitAccount {
                    name: Some("renamed-account".to_owned()),
                    ..UpdateGitAccount::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(created.audit.created_by_actor_id, actor_id);
        assert_eq!(updated.audit, created.audit);
        assert_eq!(updated.name, "renamed-account");
        assert_eq!(updated.domain, "git.example.test");
        assert_eq!(updated.transport, GitTransport::Https);
        assert_eq!(updated.auth_type, GitAuthType::Token);
        assert_eq!(
            service
                .unprotect(
                    &service
                        .store
                        .get(created.id)
                        .await
                        .unwrap()
                        .protected_configuration
                )
                .unwrap(),
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
        let mut input = CreateGitAccount {
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

    #[tokio::test]
    async fn config_reads_derive_metadata_from_credentials_without_a_second_copy() {
        let store = Arc::new(MemoryStore::default());
        let service = GitAccountService::new(store.clone(), Arc::new(ReversingProtector));
        for configuration in [
            GitAuthConfiguration::Basic {
                username: "saved-user".into(),
                password: "sentinel-password".into(),
            },
            GitAuthConfiguration::SshKey {
                username: "saved-user".into(),
                private_key: "sentinel-private-key".into(),
                passphrase: Some("sentinel-passphrase".into()),
            },
        ] {
            let mut input = token_input("unused");
            let ssh = matches!(configuration, GitAuthConfiguration::SshKey { .. });
            input.auth_type = if ssh {
                GitAuthType::SshKey
            } else {
                GitAuthType::Basic
            };
            input.transport = if ssh {
                GitTransport::Ssh
            } else {
                GitTransport::Https
            };
            input.configuration = configuration;
            let account = service
                .create(ActorId::new(Uuid::now_v7()), input)
                .await
                .unwrap();
            let metadata = service.get_config(account.id).await.unwrap().configuration;
            assert_eq!(metadata.username.as_deref(), Some("saved-user"));
            assert_eq!(metadata.has_password, !ssh);
            assert_eq!(metadata.has_private_key, ssh);
            assert_eq!(metadata.has_passphrase, ssh.then_some(true));
            assert!(
                !serde_json::to_string(&metadata)
                    .unwrap()
                    .contains("sentinel-")
            );
            {
                let mut accounts = store.accounts.lock().unwrap();
                let saved = accounts
                    .iter_mut()
                    .find(|saved| saved.id == account.id)
                    .unwrap();
                assert!(saved.protected_configuration.get("metadata").is_none());
                // Records written with the previous metadata copy remain readable;
                // the encrypted configuration is authoritative even if that copy is stale.
                saved.protected_configuration["metadata"] = json!({"username":"stale-user"});
            }
            assert_eq!(
                service.get_config(account.id).await.unwrap().configuration,
                metadata
            );
        }
    }

    #[tokio::test]
    async fn reads_redact_secrets_and_nested_updates_preserve_omitted_values() {
        let store = Arc::new(MemoryStore::default());
        let service = GitAccountService::new(store.clone(), Arc::new(ReversingProtector));
        let account = service
            .create(ActorId::new(Uuid::now_v7()), token_input("sentinel-secret"))
            .await
            .unwrap();
        let metadata = service.get_config(account.id).await.unwrap();
        assert!(metadata.configuration.has_token);
        assert!(
            !serde_json::to_string(&metadata.configuration)
                .unwrap()
                .contains("sentinel-secret")
        );
        service
            .update(
                account.id,
                serde_json::from_value(json!({"configuration":{"$type":"Token"}})).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            service.credentials(account.id).await.unwrap().configuration,
            GitAuthConfiguration::Token {
                token: "sentinel-secret".into()
            }
        );
        service
            .update(
                account.id,
                serde_json::from_value(
                    json!({"configuration":{"$type":"Token","token":"rotated"}}),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            service.credentials(account.id).await.unwrap().configuration,
            GitAuthConfiguration::Token {
                token: "rotated".into()
            }
        );
        assert!(
            service
                .update(
                    account.id,
                    serde_json::from_value(json!({"configuration":{"$type":"Token","token":""}}))
                        .unwrap()
                )
                .await
                .is_err()
        );
    }
}
