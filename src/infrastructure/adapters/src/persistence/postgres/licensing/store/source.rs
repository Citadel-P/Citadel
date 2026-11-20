use chrono::{DateTime, Utc};
use citadel_licensing::{
    CitadelInstanceIdentity, InstalledLicense, LicenseError, LicenseSource, LicenseStatus,
};
use citadel_primitives::ActorId;
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgRow};
use uuid::Uuid;

use super::storage;

pub(super) async fn load_source(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<LicenseSource, LicenseError> {
    let row = sqlx::query(
        "SELECT i.instanceid, i.createdat, l.id AS licenseid, l.rawlicense, l.fingerprint,
                l.installedat, l.installedbyactorid, l.lastvalidatedat, l.lastvalidationstatus,
                l.lastvalidationerrorcode
         FROM citadelinstanceidentity i LEFT JOIN installedlicenses l ON l.id = i.id
         WHERE i.id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    if let Some(row) = row {
        let license_id: Option<i32> = row.try_get("licenseid").map_err(storage)?;
        return Ok(LicenseSource {
            identity: instance_identity(&row)?,
            installed: license_id.map(|_| installed_license(&row)).transpose()?,
        });
    }

    let mut transaction = pool.begin().await.map_err(storage)?;
    let identity = get_or_create_identity(&mut transaction, now).await?;
    let installed = load_installed(&mut transaction, false).await?;
    transaction.commit().await.map_err(storage)?;
    Ok(LicenseSource {
        identity,
        installed,
    })
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
    instance_identity(&row)
}

fn instance_identity(row: &PgRow) -> Result<CitadelInstanceIdentity, LicenseError> {
    Ok(CitadelInstanceIdentity {
        instance_id: row.try_get("instanceid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

pub(super) async fn load_installed(
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
    row.as_ref().map(installed_license).transpose()
}

fn installed_license(row: &PgRow) -> Result<InstalledLicense, LicenseError> {
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
}
