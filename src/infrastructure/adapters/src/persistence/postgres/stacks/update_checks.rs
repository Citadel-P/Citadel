use super::*;

pub(super) async fn save(
    store: &PostgresStackRepository,
    actor: ActorId,
    administrator: bool,
    expected: &citadel_stacks::Stack,
    state: &StackUpdateState,
    git_inputs: Option<GitCheckInputs>,
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
    let inputs = git_inputs.map(|pair| vec![pair.release_revision, pair.ref_revision]);
    if sqlx::query("UPDATE stacks SET stackupdatestate=$4,rowversion=rowversion+1,
        checkedgitinputs=COALESCE($5,checkedgitinputs)
        WHERE id=$1 AND rowversion=$2 AND currentstackreleaseid=$3 AND controlstate='Idle'
        AND ($5::uuid[] IS NULL OR EXISTS (
            SELECT 1 FROM stackreleases r JOIN gitrepositoryrefs ref
              ON ref.gitrepositoryid=CASE WHEN pg_input_is_valid(r.spec->>'GitRepoId','uuid')
                  THEN (r.spec->>'GitRepoId')::uuid END AND ref.branch=r.spec->>'Branch'
            WHERE r.id=$3 AND ref.status='Healthy'
              AND ARRAY[COALESCE(r.gitcheckrevision,r.id),COALESCE(ref.gitcheckrevision,ref.id)]=$5))")
        .bind(expected.id).bind(expected.row_version).bind(expected.current_stack_release_id)
        .bind(state.to_storage_value()?).bind(inputs)
        .execute(&mut *tx).await.map_err(storage)?.rows_affected()!=1 {
        return Err(StackError::Conflict("The Stack or its Git inputs changed during the update check. Check again.".into()));
    }
    let resolution = &crate::persistence::postgres::alerts::update_resolution::stack_images(state);
    crate::persistence::postgres::alerts::update_resolution::enqueue(
        &mut tx,
        expected.id,
        resolution,
    )
    .await
    .map_err(storage)?;
    if let StackUpdateState::Git {
        recreate_stack_on_new_commit_state,
        ..
    } = state
    {
        crate::persistence::postgres::alerts::update_resolution::enqueue(
            &mut tx,
            expected.id,
            &citadel_alerts::update_resolution::VerifiedUpdate::StackGit(
                recreate_stack_on_new_commit_state
                    .current_commit_sha
                    .clone(),
            ),
        )
        .await
        .map_err(storage)?;
    }
    tx.commit().await.map_err(storage)
}
pub(super) async fn candidates(
    pool: &PgPool,
    after: Uuid,
    images: bool,
    limit: i64,
    reference: Option<Uuid>,
) -> Result<Vec<StackCheckCandidate>, StackError> {
    let limit = limit.clamp(1, 100);
    if images {
        let ids = sqlx::query_scalar("SELECT s.id FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid
            JOIN platforms p ON p.id=r.platformid WHERE s.id>$1 AND s.controlstate='Idle' AND p.status='Online'
            AND r.status IN ('Healthy','Degraded') AND r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy')
            AND s.stacksource IN ('WebEditor','Git') AND COALESCE(r.spec->>'RegistryId','') NOT IN ('','00000000-0000-0000-0000-000000000000')
            AND p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone','DockerSwarm') ORDER BY s.id LIMIT $2")
            .bind(after).bind(limit).fetch_all(pool).await.map_err(storage)?;
        return Ok(ids
            .into_iter()
            .map(|id| StackCheckCandidate {
                id,
                git_inputs: None,
            })
            .collect());
    }
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(include_str!("git_check_candidates.sql"))
        .bind(after)
        .bind(limit)
        .bind(reference)
        .fetch_all(pool)
        .await
        .map_err(storage)?;
    Ok(rows
        .into_iter()
        .map(|(id, release_revision, ref_revision)| StackCheckCandidate {
            id,
            git_inputs: Some(GitCheckInputs {
                release_revision,
                ref_revision,
            }),
        })
        .collect())
}
