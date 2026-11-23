use super::*;

pub(super) fn context(row: PgRow) -> Result<citadel_stacks::StackDriftContext, StackError> {
    Ok(citadel_stacks::StackDriftContext {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        drift_policy: StackDriftPolicy::from_storage_value(
            row.try_get("driftpolicy").map_err(storage)?,
        )?,
        control_state: row
            .try_get::<&str, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        current_stack_release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
        input_revision: Some(row.try_get("input_revision").map_err(storage)?),
        row_version: row.try_get("rowversion").map_err(storage)?,
        status: row
            .try_get::<&str, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        platform_status: row
            .try_get::<&str, _>("platform_status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        platform_type: crate::persistence::postgres::platforms::classification::platform_kind(
            row.try_get("platform_kind").map_err(storage)?,
        )
        .map_err(storage)?,
        platform_id: Some(row.try_get("platformid").map_err(storage)?),
        spec: Some(StackSpec::from_storage_value(
            row.try_get("spec").map_err(storage)?,
        )?),
        deployed_commit: row.try_get("deployed_commit").map_err(storage)?,
        previous_fingerprint: row.try_get("previous_fingerprint").map_err(storage)?,
    })
}

pub(super) async fn record(
    pool: &PgPool,
    expected: &citadel_stacks::StackDriftContext,
    status: StackReleaseStatus,
    info: ActivityEventInfo,
) -> Result<bool, StackError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    // Drift was observed outside the transaction. Never overwrite a newer apply,
    // policy edit, intentional state change or observation of another release.
    let locked = sqlx::query_scalar::<_, Uuid>(
        "SELECT s.id FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 AND s.currentstackreleaseid=$2 AND s.rowversion=$3 AND s.controlstate='Idle' AND r.status=$4 AND ($5::uuid IS NULL OR COALESCE(r.gitcheckrevision,r.id)=$5) AND p.status='Online' FOR NO KEY UPDATE OF s,r",
    )
    .bind(expected.id)
    .bind(expected.current_stack_release_id)
    .bind(expected.row_version)
    .bind(expected.status.as_str())
    .bind(expected.input_revision)
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
