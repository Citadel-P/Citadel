use super::*;

/// Pre-migration, read-only admission check. Older/fresh databases may not have
/// update tables yet, but a missing table after migration 0002 is not a clean
/// installation. Do not deserialize operation JSON: even corrupt or future
/// operation formats must retain installation ownership.
pub async fn check_startup_updates(
    pool: &PgPool,
    registered_instance: Option<Uuid>,
) -> Result<(), UpdateError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let updates_exist: bool =
        sqlx::query_scalar("SELECT to_regclass('public.instanceupdates') IS NOT NULL")
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
    if updates_exist {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM public.instanceupdates WHERE ownsinstallation)",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?;
        if owned {
            return Err(UpdateError::Conflict(
                "An instance update retains installation ownership; recovery verification is required before Core can start.".into(),
            ));
        }
    } else {
        let history_exists: bool = sqlx::query_scalar(
            "SELECT to_regclass('public.citadel_schema_migrations') IS NOT NULL",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?;
        if history_exists {
            let applied: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM public.citadel_schema_migrations WHERE id='0002' AND completed_at IS NOT NULL)",
            ).fetch_one(&mut *tx).await.map_err(storage)?;
            if applied {
                return Err(UpdateError::Conflict(
                    "Instance update state is missing from a migrated database; recovery verification is required.".into(),
                ));
            }
        }
    }
    if let Some(expected) = registered_instance {
        let actual: Option<Uuid> = sqlx::query_scalar(
            "SELECT instanceid FROM public.citadelinstanceidentity WHERE id=1 AND EXISTS(SELECT 1 FROM public.instancesetupstates WHERE id=1 AND initializedat IS NOT NULL)",
        ).fetch_optional(&mut *tx).await.map_err(storage)?;
        if expected.is_nil() || actual != Some(expected) {
            return Err(UpdateError::Conflict(
                "Update registration does not match this initialized database instance.".into(),
            ));
        }
    }
    tx.rollback().await.map_err(storage)
}
