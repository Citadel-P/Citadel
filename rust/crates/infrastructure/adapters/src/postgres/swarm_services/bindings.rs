use std::sync::Arc;

use citadel_bindings::SecretProtector;
use citadel_swarm_services::{
    ResolvedSwarmServiceBinding, ResolvedSwarmServiceBindings, SwarmServiceBindingResolverPort,
    SwarmServiceError,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::secret_value_resolver::PostgresSecretValueResolver;

#[derive(Clone)]
pub struct PostgresSwarmServiceBindingResolver {
    pool: PgPool,
    secrets: PostgresSecretValueResolver,
}

impl PostgresSwarmServiceBindingResolver {
    pub fn new(
        pool: PgPool,
        protector: Arc<dyn SecretProtector>,
    ) -> Result<Self, SwarmServiceError> {
        Ok(Self {
            pool: pool.clone(),
            secrets: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| SwarmServiceError::Storage(error.to_string()))?,
        })
    }
}

impl SwarmServiceBindingResolverPort for PostgresSwarmServiceBindingResolver {
    fn resolve<'a>(
        &'a self,
        service_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedSwarmServiceBindings, SwarmServiceError>> {
        Box::pin(async move {
            if referenced_names.is_empty() {
                return Ok(ResolvedSwarmServiceBindings::default());
            }
            let normalized = referenced_names
                .iter()
                .map(|name| name.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let rows = sqlx::query(
                r#"SELECT binding.name,binding.kind,binding.secretdeliverymode,
                          binding.value,binding.secretid
                   FROM resourcebindings binding
                   WHERE (binding.scope='Global' AND binding.resourceid IS NULL
                          OR binding.scope='SwarmService' AND binding.resourceid=$1)
                     AND lower(binding.name)=ANY($2::text[])
                   ORDER BY CASE WHEN binding.scope='Global' THEN 0 ELSE 1 END,
                            binding.createdat,binding.id"#,
            )
            .bind(service_id)
            .bind(&normalized)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            let mut effective = std::collections::BTreeMap::new();
            for row in rows {
                let name: String = row.try_get("name").map_err(storage)?;
                let secret = row.try_get::<String, _>("kind").map_err(storage)? == "Secret";
                let value = if secret {
                    if row
                        .try_get::<Option<String>, _>("secretdeliverymode")
                        .map_err(storage)?
                        .as_deref()
                        != Some("EnvironmentVariable")
                    {
                        return Err(SwarmServiceError::Validation(format!(
                            "Secret '{name}' must use environment-variable delivery for a Service."
                        )));
                    }
                    let secret_id = row
                        .try_get::<Option<Uuid>, _>("secretid")
                        .map_err(storage)?
                        .ok_or_else(|| {
                            SwarmServiceError::Validation(format!(
                                "Secret binding '{name}' has no Secret reference."
                            ))
                        })?;
                    self.secrets.resolve(secret_id).await.map_err(|error| {
                        SwarmServiceError::Validation(format!(
                            "Secret '{name}' could not be resolved: {error}"
                        ))
                    })?
                } else {
                    Zeroizing::new(
                        row.try_get::<Option<String>, _>("value")
                            .map_err(storage)?
                            .ok_or_else(|| {
                                SwarmServiceError::Validation(format!(
                                    "Variable '{name}' has no value."
                                ))
                            })?,
                    )
                };
                effective.insert(
                    name.to_ascii_lowercase(),
                    ResolvedSwarmServiceBinding {
                        name,
                        value,
                        secret,
                    },
                );
            }
            Ok(ResolvedSwarmServiceBindings {
                entries: effective.into_values().collect(),
            })
        })
    }
}

fn storage(error: impl std::fmt::Display) -> SwarmServiceError {
    SwarmServiceError::Storage(error.to_string())
}
