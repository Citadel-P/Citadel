use std::collections::BTreeMap;
use std::sync::Arc;

use citadel_bindings::SecretProtector;
use citadel_stacks::{
    ResolvedStackBindings, ResourceBindingSnapshot, StackBinding, StackBindingResolverPort,
    StackError,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver;

#[derive(Clone)]
pub struct PostgresStackBindingResolver {
    pool: PgPool,
    secrets: PostgresSecretValueResolver,
}

impl PostgresStackBindingResolver {
    pub fn new(pool: PgPool, protector: Arc<dyn SecretProtector>) -> Result<Self, StackError> {
        Ok(Self {
            pool: pool.clone(),
            secrets: PostgresSecretValueResolver::new(pool, protector)
                .map_err(|error| StackError::Storage(error.to_string()))?,
        })
    }
}

impl StackBindingResolverPort for PostgresStackBindingResolver {
    fn resolve<'a>(
        &'a self,
        stack_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedStackBindings, StackError>> {
        Box::pin(async move {
            if referenced_names.is_empty() {
                return Ok(ResolvedStackBindings::default());
            }
            let normalized = referenced_names
                .iter()
                .map(|name| name.to_ascii_lowercase())
                .collect::<Vec<_>>();
            let rows = sqlx::query(
                r#"SELECT b.name,b.kind,b.scope,b.value,b.secretid,b.secretdeliverymode,b.targetpath
                   FROM resourcebindings b
                   WHERE (b.scope='Global' AND b.resourceid IS NULL
                          OR b.scope='Stack' AND b.resourceid=$1)
                     AND lower(b.name)=ANY($2::text[])
                   ORDER BY CASE WHEN b.scope='Global' THEN 0 ELSE 1 END,b.createdat,b.id"#,
            )
            .bind(stack_id)
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
                        return Err(StackError::Validation(format!(
                            "Secret '{name}' must use environment-variable delivery for this Stack."
                        )));
                    }
                    let secret_id = secret_id.ok_or_else(|| {
                        StackError::Validation(format!(
                            "Secret binding '{name}' has no Secret reference."
                        ))
                    })?;
                    self.secrets.resolve(secret_id).await.map_err(|error| {
                        StackError::Validation(format!(
                            "Secret '{name}' could not be resolved: {error}"
                        ))
                    })?
                } else {
                    Zeroizing::new(
                        row.try_get::<Option<String>, _>("value")
                            .map_err(storage)?
                            .ok_or_else(|| {
                                StackError::Validation(format!("Variable '{name}' has no value."))
                            })?,
                    )
                };
                effective.insert(
                    name.to_ascii_lowercase(),
                    StackBinding {
                        name: name.clone(),
                        value,
                        secret,
                        snapshot: ResourceBindingSnapshot {
                            name,
                            kind,
                            scope,
                            value: if secret {
                                "********".to_owned()
                            } else {
                                row.try_get::<Option<String>, _>("value")
                                    .map_err(storage)?
                                    .unwrap_or_default()
                            },
                            secret_id,
                            secret_delivery_mode: delivery,
                            target_path,
                        },
                    },
                );
            }
            Ok(ResolvedStackBindings {
                entries: effective.into_values().collect(),
            })
        })
    }
}

fn storage(error: impl std::fmt::Display) -> StackError {
    StackError::Storage(error.to_string())
}
