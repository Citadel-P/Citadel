use std::collections::BTreeMap;
use std::sync::Arc;

use citadel_bindings::SecretProtector;
use citadel_deployments::{
    DeploymentBindingResolverPort, DeploymentBindingSnapshot, DeploymentError,
    ResolvedDeploymentBinding, ResolvedDeploymentBindings,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver;

#[derive(Clone)]
pub struct PostgresDeploymentBindingResolver {
    pool: PgPool,
    secrets: PostgresSecretValueResolver,
}

impl PostgresDeploymentBindingResolver {
    pub fn new(pool: PgPool, protector: Arc<dyn SecretProtector>) -> Result<Self, DeploymentError> {
        Ok(Self {
            pool: pool.clone(),
            secrets: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| DeploymentError::Storage(error.to_string()))?,
        })
    }
}

impl DeploymentBindingResolverPort for PostgresDeploymentBindingResolver {
    fn resolve<'a>(
        &'a self,
        deployment_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedDeploymentBindings, DeploymentError>> {
        Box::pin(async move {
            if referenced_names.is_empty() {
                return Ok(ResolvedDeploymentBindings::default());
            }
            let normalized = referenced_names
                .iter()
                .map(|name| name.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let rows = sqlx::query(
                r#"SELECT b.name,b.kind,b.scope,b.value,b.secretid,b.secretdeliverymode,b.targetpath
                   FROM resourcebindings b
                   WHERE (b.scope='Global' AND b.resourceid IS NULL
                          OR b.scope='Deployment' AND b.resourceid=$1)
                     AND lower(b.name)=ANY($2::text[])
                   ORDER BY CASE WHEN b.scope='Global' THEN 0 ELSE 1 END,b.createdat,b.id"#,
            )
            .bind(deployment_id)
            .bind(&normalized)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

            let mut effective = BTreeMap::new();
            for row in rows {
                let name: String = row.try_get("name").map_err(storage)?;
                let kind: String = row.try_get("kind").map_err(storage)?;
                let scope: String = row.try_get("scope").map_err(storage)?;
                let secret_id: Option<Uuid> = row.try_get("secretid").map_err(storage)?;
                let delivery: Option<String> =
                    row.try_get("secretdeliverymode").map_err(storage)?;
                let target_path: Option<String> = row.try_get("targetpath").map_err(storage)?;
                let secret = kind == "Secret";
                let value = if secret {
                    if delivery.as_deref() != Some("EnvironmentVariable") {
                        return Err(DeploymentError::Validation(format!(
                            "Secret '{name}' must use environment-variable delivery for a Deployment."
                        )));
                    }
                    let secret_id = secret_id.ok_or_else(|| {
                        DeploymentError::Validation(format!(
                            "Secret binding '{name}' has no Secret reference."
                        ))
                    })?;
                    self.secrets.resolve(secret_id).await.map_err(|error| {
                        DeploymentError::Validation(format!(
                            "Secret '{name}' could not be resolved: {error}"
                        ))
                    })?
                } else {
                    Zeroizing::new(
                        row.try_get::<Option<String>, _>("value")
                            .map_err(storage)?
                            .ok_or_else(|| {
                                DeploymentError::Validation(format!(
                                    "Variable '{name}' has no value."
                                ))
                            })?,
                    )
                };
                let snapshot = DeploymentBindingSnapshot {
                    name: name.clone(),
                    kind: kind.clone(),
                    scope,
                    value: if secret {
                        "********".to_owned()
                    } else {
                        value.to_string()
                    },
                    secret_id,
                    secret_delivery_mode: delivery,
                    target_path,
                };
                effective.insert(
                    name.to_ascii_lowercase(),
                    ResolvedDeploymentBinding {
                        name,
                        value,
                        secret,
                        snapshot,
                    },
                );
            }
            Ok(ResolvedDeploymentBindings {
                entries: effective.into_values().collect(),
            })
        })
    }
}

fn storage(error: impl std::fmt::Display) -> DeploymentError {
    DeploymentError::Storage(error.to_string())
}
