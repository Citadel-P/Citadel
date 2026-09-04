use std::collections::BTreeMap;
use std::sync::Arc;

use citadel_deployments::{
    DeploymentBindingResolverPort, DeploymentBindingSnapshot, DeploymentError,
    ResolvedDeploymentBinding, ResolvedDeploymentBindings,
};
use citadel_resources::ResourceSecretProtector;
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct PostgresDeploymentBindingResolver {
    pool: PgPool,
    protector: Arc<dyn ResourceSecretProtector>,
}

impl PostgresDeploymentBindingResolver {
    #[must_use]
    pub fn new(pool: PgPool, protector: Arc<dyn ResourceSecretProtector>) -> Self {
        Self { pool, protector }
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
                r#"SELECT b.name,b.kind,b.scope,b.value,b.secretid,b.secretdeliverymode,b.targetpath,
                          secret.providertype,internal.encryptedvalue
                   FROM resourcebindings b
                   LEFT JOIN secretdefinitions secret ON secret.id=b.secretid
                   LEFT JOIN internalsecretvalues internal ON internal.secretid=secret.id
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
                    let provider: Option<String> = row.try_get("providertype").map_err(storage)?;
                    if provider.as_deref() != Some("InternalEncrypted") {
                        return Err(DeploymentError::Validation(format!(
                            "External Secret '{name}' cannot be resolved until external provider execution migrates to Rust."
                        )));
                    }
                    let envelope: Option<String> =
                        row.try_get("encryptedvalue").map_err(storage)?;
                    let envelope = envelope.ok_or_else(|| {
                        DeploymentError::Validation(format!(
                            "Secret '{name}' has no encrypted value."
                        ))
                    })?;
                    let plaintext = self.protector.unprotect(&envelope).map_err(|_| {
                        DeploymentError::Validation(format!(
                            "Secret '{name}' could not be decrypted."
                        ))
                    })?;
                    let text = String::from_utf8(plaintext.to_vec()).map_err(|_| {
                        DeploymentError::Validation(format!(
                            "Secret '{name}' does not contain valid UTF-8 text."
                        ))
                    })?;
                    Zeroizing::new(text)
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
