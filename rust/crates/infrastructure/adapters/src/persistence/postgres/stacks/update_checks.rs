use super::*;

pub(super) async fn save(
    store: &PostgresStackRepository,
    actor: ActorId,
    administrator: bool,
    expected: &citadel_stacks::Stack,
    state: &StackUpdateState,
) -> Result<(), StackError> {
    let mut tx = store.pool.begin().await.map_err(storage)?;
    ensure_access(
        &mut tx,
        actor,
        administrator,
        expected.id,
        policy::WriteStack::REQUIREMENT,
    )
    .await?;
    if sqlx::query("UPDATE stacks SET stackupdatestate=$4,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND currentstackreleaseid=$3 AND controlstate='Idle'")
        .bind(expected.id).bind(expected.row_version).bind(expected.current_stack_release_id).bind(state.to_storage_value()?)
        .execute(&mut *tx).await.map_err(storage)?.rows_affected()!=1 {
        return Err(StackError::Conflict("The Stack changed during the update check. Check again.".into()));
    }
    tx.commit().await.map_err(storage)
}
pub(super) async fn candidates(
    pool: &PgPool,
    after: Uuid,
    images: bool,
    limit: i64,
) -> Result<Vec<Uuid>, StackError> {
    sqlx::query_scalar("SELECT s.id FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id>$1 AND s.controlstate='Idle' AND p.status='Online' AND r.status IN ('Healthy','Degraded') AND r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy') AND (($2 AND s.stacksource='WebEditor' AND p.platformdescriptor->>'$type'='Docker') OR (s.stacksource='Git' AND COALESCE(r.spec->>'CommitSha','')='' AND EXISTS(SELECT 1 FROM gitrepositoryrefs ref WHERE ref.gitrepositoryid::text=r.spec->>'GitRepoId' AND ref.branch=r.spec->>'Branch' AND ref.status='Healthy' AND ref.lastsyncedat>CASE WHEN s.stackupdatestate->'RecreateStackOnNewCommitState'->>'LastCheckedAt' ~ '^[0-9]{4}-' THEN (s.stackupdatestate->'RecreateStackOnNewCommitState'->>'LastCheckedAt')::timestamptz ELSE '-infinity'::timestamptz END))) ORDER BY s.id LIMIT $3")
        .bind(after).bind(images).bind(limit.clamp(1,100)).fetch_all(pool).await.map_err(storage)
}
