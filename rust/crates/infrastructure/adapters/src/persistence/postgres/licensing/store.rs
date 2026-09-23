use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use citadel_activities::ActivityEvent;

use citadel_licensing::LicenseSource;
use citadel_licensing::LicenseStore;
use citadel_licensing::LicenseValidationPersistence;
use citadel_licensing::LicenseVerifier;
use citadel_licensing::build_state;

use citadel_deployments::{DeploymentEntitlementPort, DeploymentError};

use citadel_identity::{EntitlementService, SYSTEM_ACTOR_ID};
use citadel_licensing::{LicenseChange, LicenseError};
mod activity;

use citadel_licensing::{
    CitadelInstanceIdentity, InstalledLicense, LicenseCapability, LicenseStatus,
    LicenseVerificationResult,
};

use citadel_primitives::ActorId;

use futures_util::future::BoxFuture;

use sqlx::{PgPool, Postgres, Row, Transaction};

use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;

use crate::security::licensing::verifier::Ed25519LicenseVerifier;
use crate::security::licensing::verifier::temporal_status;

#[derive(Clone)]
pub struct PostgresLicenseStore {
    pool: PgPool,
}

impl PostgresLicenseStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl LicenseStore for PostgresLicenseStore {
    fn load(&self, now: DateTime<Utc>) -> BoxFuture<'_, Result<LicenseSource, LicenseError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let installed = load_installed(&mut transaction, false).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(LicenseSource {
                identity,
                installed,
            })
        })
    }

    fn install<'a>(
        &'a self,
        expected_fingerprint: Option<&'a str>,
        installed: &'a InstalledLicense,
        actor_id: ActorId,
        info: &'a LicenseChange,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, LicenseError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            if current.as_ref().map(|license| license.fingerprint.as_str()) != expected_fingerprint
            {
                return Ok(false);
            }
            sqlx::query(
                r#"
INSERT INTO installedlicenses (
    id, rawlicense, fingerprint, installedat, installedbyactorid,
    lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode)
VALUES (1, $1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (id) DO UPDATE SET
    rawlicense = EXCLUDED.rawlicense,
    fingerprint = EXCLUDED.fingerprint,
    installedat = EXCLUDED.installedat,
    installedbyactorid = EXCLUDED.installedbyactorid,
    lastvalidatedat = EXCLUDED.lastvalidatedat,
    lastvalidationstatus = EXCLUDED.lastvalidationstatus,
    lastvalidationerrorcode = EXCLUDED.lastvalidationerrorcode
"#,
            )
            .bind(&installed.raw_license)
            .bind(&installed.fingerprint)
            .bind(installed.installed_at)
            .bind(installed.installed_by_actor_id.map(ActorId::value))
            .bind(installed.last_validated_at)
            .bind(
                installed
                    .last_validation_status
                    .map(LicenseStatus::as_database_str),
            )
            .bind(&installed.last_validation_error_code)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            insert_license_activity(&mut transaction, &identity, actor_id, info.clone(), now)
                .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn remove<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        actor_id: ActorId,
        info: &'a LicenseChange,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, LicenseError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            if current.as_ref().map(|license| license.fingerprint.as_str())
                != Some(expected_fingerprint)
            {
                return Ok(false);
            }
            sqlx::query("DELETE FROM installedlicenses WHERE id = 1")
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            insert_license_activity(&mut transaction, &identity, actor_id, info.clone(), now)
                .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn persist_validation<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        status: LicenseStatus,
        error_code: Option<&'a str>,
        transition_activity: Option<&'a LicenseChange>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<LicenseValidationPersistence, LicenseError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            let Some(current) = current else {
                return Ok(LicenseValidationPersistence::SourceChanged);
            };
            if current.fingerprint != expected_fingerprint {
                return Ok(LicenseValidationPersistence::SourceChanged);
            }
            let transitioned = current.last_validation_status != Some(status);
            sqlx::query(
                "UPDATE installedlicenses SET lastvalidatedat = $1, lastvalidationstatus = $2, lastvalidationerrorcode = $3 WHERE id = 1",
            )
            .bind(now)
            .bind(status.as_database_str())
            .bind(error_code)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            if transitioned && let Some(info) = transition_activity {
                insert_license_activity(
                    &mut transaction,
                    &identity,
                    ActorId::new(SYSTEM_ACTOR_ID),
                    info.clone(),
                    now,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(if transitioned {
                LicenseValidationPersistence::Transitioned
            } else {
                LicenseValidationPersistence::Unchanged
            })
        })
    }
}

#[derive(Clone)]
struct CachedVerification {
    fingerprint: String,
    verification: LicenseVerificationResult,
}

pub struct PostgresLicenseEntitlementService {
    store: Arc<PostgresLicenseStore>,
    verifier: Arc<Ed25519LicenseVerifier>,
    cached: RwLock<Option<CachedVerification>>,
}

impl PostgresLicenseEntitlementService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self::with(
            Arc::new(PostgresLicenseStore::new(pool)),
            Arc::new(Ed25519LicenseVerifier::default()),
        )
    }

    #[must_use]
    pub fn with(store: Arc<PostgresLicenseStore>, verifier: Arc<Ed25519LicenseVerifier>) -> Self {
        Self {
            store,
            verifier,
            cached: RwLock::new(None),
        }
    }

    pub fn invalidate(&self) {
        *self.cached.write().expect("license cache lock poisoned") = None;
    }

    async fn enabled(&self, capability: LicenseCapability) -> Result<bool, LicenseError> {
        let now = Utc::now();
        let source = self.store.load(now).await?;
        let Some(installed) = &source.installed else {
            return Ok(false);
        };
        let cached = self
            .cached
            .read()
            .expect("license cache lock poisoned")
            .as_ref()
            .filter(|cached| cached.fingerprint == installed.fingerprint)
            .cloned();
        let mut verification = if let Some(cached) = cached {
            cached.verification
        } else {
            let verification = self
                .verifier
                .verify(&installed.raw_license, &source.identity, now);
            *self.cached.write().expect("license cache lock poisoned") = Some(CachedVerification {
                fingerprint: installed.fingerprint.clone(),
                verification: verification.clone(),
            });
            verification
        };
        if let Some(license) = &verification.license {
            verification.status = temporal_status(&license.payload, now);
        }
        Ok(
            build_state(&source.identity, Some(installed), Some(&verification))
                .capability_enabled(capability),
        )
    }
}

impl EntitlementService for PostgresLicenseEntitlementService {
    fn custom_access_control_enabled(
        &self,
    ) -> BoxFuture<'_, Result<bool, citadel_identity::IdentityError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::CustomAccessControl)
                .await
                .map_err(|error| citadel_identity::IdentityError::Storage(error.to_string()))
        })
    }
}

impl DeploymentEntitlementPort for PostgresLicenseEntitlementService {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, DeploymentError>> {
        Box::pin(async move {
            self.enabled(capability)
                .await
                .map_err(|error| DeploymentError::Storage(error.to_string()))
        })
    }
}

impl citadel_builds::BuildEntitlements for PostgresLicenseEntitlementService {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, citadel_builds::BuildError>> {
        Box::pin(async move {
            self.enabled(capability)
                .await
                .map_err(|error| citadel_builds::BuildError::Storage(error.to_string()))
        })
    }
}

impl citadel_stacks::StackEntitlements for PostgresLicenseEntitlementService {
    fn operational_guardrails(&self) -> BoxFuture<'_, Result<bool, citadel_stacks::StackError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::OperationalGuardrails)
                .await
                .map_err(|error| citadel_stacks::StackError::Storage(error.to_string()))
        })
    }
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, citadel_stacks::StackError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::AutomatedOperations)
                .await
                .map_err(|error| citadel_stacks::StackError::Storage(error.to_string()))
        })
    }
}

impl citadel_swarm_services::ServiceAutomationEntitlements for PostgresLicenseEntitlementService {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, citadel_swarm_services::SwarmServiceError>> {
        Box::pin(async move {
            self.enabled(capability).await.map_err(|error| {
                citadel_swarm_services::SwarmServiceError::Storage(error.to_string())
            })
        })
    }
}

impl citadel_automation::AutomationEntitlements for PostgresLicenseEntitlementService {
    fn automated_operations(
        &self,
    ) -> BoxFuture<'_, Result<bool, citadel_automation::AutomationError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::AutomatedOperations)
                .await
                .map_err(|error| citadel_automation::AutomationError::Storage(error.to_string()))
        })
    }
}

impl citadel_backups::BackupEntitlements for PostgresLicenseEntitlementService {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, citadel_backups::BackupError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::AutomatedOperations)
                .await
                .map_err(|error| citadel_backups::BackupError::Storage(error.to_string()))
        })
    }
}

impl citadel_alerts::AlertEntitlements for PostgresLicenseEntitlementService {
    fn advanced_alerting(&self) -> BoxFuture<'_, Result<bool, citadel_alerts::AlertError>> {
        Box::pin(async move {
            self.enabled(LicenseCapability::AdvancedAlerting)
                .await
                .map_err(|error| citadel_alerts::AlertError::Storage(error.to_string()))
        })
    }
}

pub(crate) async fn get_or_create_identity(
    transaction: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
) -> Result<CitadelInstanceIdentity, LicenseError> {
    let candidate = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO citadelinstanceidentity (id, instanceid, createdat) VALUES (1, $1, $2) ON CONFLICT (id) DO NOTHING",
    )
    .bind(candidate)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    let row = sqlx::query(
        "SELECT instanceid, createdat FROM citadelinstanceidentity WHERE id = 1 LIMIT 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(CitadelInstanceIdentity {
        instance_id: row.try_get("instanceid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

async fn load_installed(
    transaction: &mut Transaction<'_, Postgres>,
    locked: bool,
) -> Result<Option<InstalledLicense>, LicenseError> {
    let suffix = if locked { " FOR UPDATE" } else { "" };
    let query = format!(
        "SELECT rawlicense, fingerprint, installedat, installedbyactorid, lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode FROM installedlicenses WHERE id = 1{suffix}"
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?;
    row.map(|row| {
        let last_validation_status: Option<String> =
            row.try_get("lastvalidationstatus").map_err(storage)?;
        Ok(InstalledLicense {
            raw_license: row.try_get("rawlicense").map_err(storage)?,
            fingerprint: row.try_get("fingerprint").map_err(storage)?,
            installed_at: row.try_get("installedat").map_err(storage)?,
            installed_by_actor_id: row
                .try_get::<Option<Uuid>, _>("installedbyactorid")
                .map_err(storage)?
                .map(ActorId::new),
            last_validated_at: row.try_get("lastvalidatedat").map_err(storage)?,
            last_validation_status: last_validation_status
                .as_deref()
                .and_then(LicenseStatus::from_database_str),
            last_validation_error_code: row.try_get("lastvalidationerrorcode").map_err(storage)?,
        })
    })
    .transpose()
}

async fn lock_license_mutations(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), LicenseError> {
    sqlx::query("LOCK TABLE installedlicenses IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(())
}

async fn insert_license_activity(
    transaction: &mut Transaction<'_, Postgres>,
    identity: &CitadelInstanceIdentity,
    actor_id: ActorId,
    info: LicenseChange,
    now: DateTime<Utc>,
) -> Result<(), LicenseError> {
    let activity = ActivityEvent::new_license_event(
        identity.instance_id,
        actor_id,
        activity::activity(info),
        now,
    )
    .map_err(storage)?;
    insert_activity(transaction, &activity)
        .await
        .map_err(storage)
}

fn storage(error: impl std::fmt::Display) -> LicenseError {
    LicenseError::Storage(error.to_string())
}
