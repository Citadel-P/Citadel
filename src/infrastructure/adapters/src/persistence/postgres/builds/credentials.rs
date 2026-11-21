use std::sync::Arc;

use citadel_bindings::SecretProtector;

use citadel_builds::{
    BuildError, BuildRegistryCredentialResolver, BuildRegistryCredentials, BuildSecretResolver,
};

use futures_util::future::BoxFuture;

use serde_json::Value;

use sqlx::PgPool;

use uuid::Uuid;

use zeroize::Zeroizing;

use crate::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver;

#[derive(Clone)]
pub struct PostgresBuildSecretResolver {
    resolver: PostgresSecretValueResolver,
}

impl PostgresBuildSecretResolver {
    pub fn with_resolver(resolver: PostgresSecretValueResolver) -> Self {
        Self { resolver }
    }

    pub fn new(pool: PgPool, protector: Arc<dyn SecretProtector>) -> Result<Self, BuildError> {
        Ok(Self {
            resolver: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| BuildError::Storage(error.to_string()))?,
        })
    }
}

impl BuildSecretResolver for PostgresBuildSecretResolver {
    fn resolve(
        &self,
        secret_id: Uuid,
        build_id: Uuid,
    ) -> BoxFuture<'_, Result<Zeroizing<String>, BuildError>> {
        Box::pin(async move {
            self.resolver
                .resolve(secret_id, "Build", build_id)
                .await
                .map_err(|error| BuildError::Validation(error.to_string()))
        })
    }
}

#[derive(Clone)]
pub struct PostgresBuildRegistryCredentialResolver {
    pool: PgPool,
}

impl PostgresBuildRegistryCredentialResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl BuildRegistryCredentialResolver for PostgresBuildRegistryCredentialResolver {
    fn resolve(
        &self,
        registry_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRegistryCredentials>, BuildError>> {
        Box::pin(async move {
            let configuration: Value = sqlx::query_scalar(
                "SELECT configuration FROM registries WHERE id=$1 AND status<>'Disabled'",
            )
            .bind(registry_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(build_storage)?
            .ok_or_else(|| BuildError::Validation("Build Registry was not found.".to_owned()))?;
            Ok(citadel_registries::RegistryConfiguration(&configuration)
                .user_password()
                .map(|(username, password)| BuildRegistryCredentials {
                    username: username.into(),
                    password: Zeroizing::new(password.into()),
                }))
        })
    }
}

fn build_storage(error: impl std::fmt::Display) -> BuildError {
    BuildError::Storage(error.to_string())
}
