use super::*;

pub(super) async fn record(
    pool: &PgPool,
    expected: &citadel_stacks::Stack,
    status: StackReleaseStatus,
    info: ActivityEventInfo,
) -> Result<bool, StackError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    // Drift was observed outside the transaction. Never overwrite a newer apply,
    // policy edit, intentional state change or observation of another release.
    let locked = sqlx::query_scalar::<_, Uuid>(
        "SELECT s.id FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 AND s.currentstackreleaseid=$2 AND s.rowversion=$3 AND s.controlstate='Idle' AND r.status=$4 AND p.status='Online' FOR NO KEY UPDATE OF s,r",
    )
    .bind(expected.id)
    .bind(expected.current_stack_release_id)
    .bind(expected.row_version)
    .bind(expected.status.as_str())
    .fetch_optional(&mut *tx).await.map_err(storage)?;
    if locked.is_none() {
        return Ok(false);
    }
    sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=$1")
        .bind(expected.current_stack_release_id)
        .bind(status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("UPDATE stacks SET rowversion=rowversion+1 WHERE id=$1")
        .bind(expected.id)
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let activity_status = if matches!(info, ActivityEventInfo::StackDriftResolved { .. }) {
        ActivityStatus::Success
    } else {
        ActivityStatus::Warning
    };
    insert_stack_activity(
        &mut tx,
        expected.id,
        &expected.name,
        expected.platform_id.ok_or(StackError::NotFound)?,
        ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
        info,
        activity_status,
    )
    .await?;
    tx.commit().await.map_err(storage)?;
    Ok(true)
}
