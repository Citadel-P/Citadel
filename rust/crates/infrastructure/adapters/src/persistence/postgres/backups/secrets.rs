use std::sync::Arc;

use citadel_backups::*;

use citadel_bindings::SecretProtector;

use futures_util::future::BoxFuture;

use sqlx::PgPool;

use uuid::Uuid;

use zeroize::Zeroizing;

use crate::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver;

#[derive(Clone)]
pub struct PostgresBackupSecretResolver {
    resolver: PostgresSecretValueResolver,
}

impl PostgresBackupSecretResolver {
    pub fn new(pool: PgPool, protector: Arc<dyn SecretProtector>) -> Result<Self, BackupError> {
        Ok(Self {
            resolver: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| BackupError::Storage(error.to_string()))?,
        })
    }
}

impl BackupSecretResolver for PostgresBackupSecretResolver {
    fn resolve(&self, id: Uuid) -> BoxFuture<'_, Result<Zeroizing<String>, BackupError>> {
        Box::pin(async move {
            self.resolver
                .resolve(id)
                .await
                .map_err(|error| BackupError::Validation(error.to_string()))
        })
    }
}
