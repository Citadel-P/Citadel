//! Image deletion is never retried. Re-read Docker after partial/ambiguous failures.
use citadel_platforms::{
    PlatformInventoryPort, RuntimeCapabilityError, RuntimeErrorKind, images::ImageDeletionPort,
};
use sqlx::{PgPool, Row};
use std::{collections::BTreeMap, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn delete(
    pool: &PgPool,
    platform_id: Uuid,
    ids: &[String],
    force: bool,
    no_prune: bool,
    mutation: &dyn ImageDeletionPort,
    inventory: &dyn PlatformInventoryPort,
) -> Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let rows = sqlx::query("SELECT id,dockerimageid,controlstate,controlstartedat FROM images WHERE platformid=$1 AND dockerimageid=ANY($2) ORDER BY id FOR UPDATE")
        .bind(platform_id).bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
    if rows.len() != ids.len() {
        return Err(error(
            RuntimeErrorKind::NotFound,
            "One or more images were not found on this Platform.",
        ));
    }
    let started = chrono::Utc::now().timestamp_millis();
    // A prior process can die mid-operation. Expired claims are released only
    // for an explicit new request, never by replaying a destructive command.
    if rows.iter().any(|row| {
        row.get::<Option<String>, _>("controlstate").as_deref() == Some("Processing")
            && row
                .get::<Option<i64>, _>("controlstartedat")
                .is_none_or(|at| at > started - 300_000)
    }) {
        return Err(error(
            RuntimeErrorKind::Conflict,
            "An Image is already processing another operation.",
        ));
    }
    let claimed: Vec<Uuid> = rows.iter().map(|row| row.get("id")).collect();
    sqlx::query("UPDATE images SET controlstate='Processing',controlstartedat=$2,rowversion=rowversion+1 WHERE id=ANY($1)")
        .bind(&claimed).bind(started).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;

    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        let mut results = Vec::new();
        for id in ids {
            results.extend(mutation.delete_image(id, force, no_prune, &cancel).await?);
        }
        Ok(results)
    })
    .await
    .unwrap_or_else(|_| {
        Err(error(
            RuntimeErrorKind::Timeout,
            "Image deletion timed out. Refresh inventory before retrying.",
        ))
    });

    // Even success can mean an untagged image, rather than removal of its data.
    // Trust a fresh inventory, not the requested ID list or a partial response.
    let observed =
        tokio::time::timeout(Duration::from_secs(15), inventory.list_images(&cancel)).await;
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(platform_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?;
    if let Ok(Ok(images)) = observed {
        let remaining: Vec<_> = images.iter().map(|image| image.id.as_str()).collect();
        sqlx::query("DELETE FROM images WHERE id=ANY($1) AND controlstartedat=$2 AND NOT (dockerimageid=ANY($3))")
            .bind(&claimed).bind(started).bind(&remaining).execute(&mut *tx).await.map_err(storage)?;
    }
    sqlx::query("UPDATE images SET controlstate='Idle',controlstartedat=NULL,rowversion=rowversion+1 WHERE id=ANY($1) AND controlstartedat=$2")
        .bind(&claimed).bind(started).execute(&mut *tx).await.map_err(storage)?;
    sqlx::query("UPDATE platforms SET imagecount=(SELECT count(*) FROM images WHERE platformid=$1) WHERE id=$1")
        .bind(platform_id).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    result
}

fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, error.to_string(), false)
}
