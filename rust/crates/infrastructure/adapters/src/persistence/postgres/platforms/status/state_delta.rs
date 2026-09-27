//! Narrow lifecycle writes. No inventory serialization, insertion or ownership joins.
use super::*;
use citadel_platforms::containers::ContainerStatePatch;
use citadel_platforms::jobs::{ContainerDeltaResult, ContainerStateDelta};
use citadel_runtime::runtime_metrics::RuntimeWork;
use std::collections::BTreeSet;

const MAX_DELTAS: usize = 256;

pub async fn container_state_deltas_committed(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    deltas: &[ContainerStateDelta],
) -> Result<Vec<ContainerDeltaResult>> {
    if deltas.is_empty() {
        return Ok(vec![]);
    }
    let write = ProjectionWrite::begin(platform, node, ProjectionKind::Containers).await;
    let mut tx = pool.begin().await?;
    let results = container_state_deltas_in(pool, &mut tx, platform, node, deltas).await?;
    tx.commit().await?;
    for result in &results {
        super::super::runtime_index::committed_identity(
            pool,
            platform,
            node,
            &result.docker_id,
            result.container_id,
        );
    }
    write.committed();
    super::super::containers::coordination::state_batch(pool, platform, node, deltas, &results);
    reconcile_state_deployments(pool, platform, &results).await?;
    Ok(results)
}

/// Parent writes run after releasing the projection transaction/locks. Each
/// external parent is reconciled once; a durable operation owns its own finish.
pub(crate) async fn reconcile_state_deployments(
    pool: &PgPool,
    platform: Uuid,
    results: &[ContainerDeltaResult],
) -> Result<()> {
    let ids: BTreeSet<_> = results
        .iter()
        .filter(|r| r.changed && !r.defer_parent_effects)
        .filter_map(|r| r.deployment_id)
        .collect();
    if !ids.is_empty() {
        reconcile_deployments(
            pool,
            platform,
            Some(&ids.into_iter().collect::<Vec<_>>()),
            true,
        )
        .await?;
    }
    Ok(())
}

/// Caller owns the projection scope and (for Edge) has validated its session.
pub(crate) async fn container_state_deltas_in(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    deltas: &[ContainerStateDelta],
) -> Result<Vec<ContainerDeltaResult>> {
    if deltas.len() > MAX_DELTAS
        || deltas
            .iter()
            .any(|d| d.docker_id.is_empty() || d.observed_at < 0 || d.observed_at_millis < 0)
        || deltas
            .iter()
            .map(|d| &d.docker_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != deltas.len()
    {
        return Err(sqlx::Error::Protocol(
            "Invalid or oversized container state delta batch".into(),
        ));
    }
    RuntimeWork::ContainerStateDeltaInput.units(deltas.len() as u64);
    let _batch = RuntimeWork::ContainerStateDeltaBatch.start();
    RuntimeWork::ContainerStateDeltaBatch.units(deltas.len() as u64);
    let _persist = RuntimeWork::ContainerStateDeltaPersist.start();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut **tx)
        .await?;
    let ids: Vec<_> = deltas.iter().map(|d| d.docker_id.clone()).collect();
    let states: Vec<_> = deltas.iter().map(|d| d.state.as_str()).collect();
    let times: Vec<_> = deltas.iter().map(|d| d.observed_at).collect();
    let hints: Vec<_> = ids
        .iter()
        .map(|id| super::super::runtime_index::hint(pool, platform, node, id))
        .collect();
    let mut rows = sqlx::query(STATE_DELTAS)
        .bind(platform)
        .bind(node)
        .bind(&ids)
        .bind(&states)
        .bind(&times)
        .bind(&hints)
        .fetch_all(&mut **tx)
        .await?;
    // An identity hint may lag another writer. Retry only those missing hints,
    // still checking platform/node/Docker ID in SQL; never enumerate siblings.
    let missing: Vec<_> = ids
        .iter()
        .enumerate()
        .filter(|(i, id)| {
            hints[*i].is_some()
                && !rows
                    .iter()
                    .any(|r| r.get::<String, _>("dockercontainerid") == **id)
        })
        .map(|(i, _)| i)
        .collect();
    if !missing.is_empty() {
        RuntimeWork::RuntimeIdentityFallback.units(missing.len() as u64);
        rows.extend(
            sqlx::query(STATE_DELTAS)
                .bind(platform)
                .bind(node)
                .bind(missing.iter().map(|&i| ids[i].clone()).collect::<Vec<_>>())
                .bind(missing.iter().map(|&i| states[i]).collect::<Vec<_>>())
                .bind(missing.iter().map(|&i| times[i]).collect::<Vec<_>>())
                .bind(vec![None::<Uuid>; missing.len()])
                .fetch_all(&mut **tx)
                .await?,
        );
    }
    let mut results = Vec::with_capacity(deltas.len());
    let mut stacks = BTreeSet::new();
    let mut degraded_stacks = BTreeSet::new();
    let mut drift = BTreeSet::new();
    let mut prune = false;
    for delta in deltas {
        let Some(row) = rows
            .iter()
            .find(|r| r.get::<String, _>("dockercontainerid") == delta.docker_id)
        else {
            RuntimeWork::ContainerStateDeltaUnknownIdentity.units(1);
            results.push(ContainerDeltaResult {
                docker_id: delta.docker_id.clone(),
                ..Default::default()
            });
            continue;
        };
        let changed: bool = row.try_get("changed")?;
        let owned = row.get::<String, _>("controlstate") == "Processing"
            && row.get::<Option<Uuid>, _>("containeroperationid").is_some();
        let stack = row.get::<Option<Uuid>, _>("stackid");
        let deployment = row.get::<Option<Uuid>, _>("deploymentid");
        let swarm = row.get::<bool, _>("isswarmtask");
        if changed {
            // Runtime state commits even during an operation. Its parent effects
            // and drift decisions belong exclusively to durable command finish.
            if owned || (stack.is_none() && deployment.is_none()) {
                RuntimeWork::ContainerParentReconcileSkipped.units(1);
            }
            if !owned {
                if let Some(stack) = stack {
                    if matches!(
                        delta.state,
                        citadel_platforms::jobs::ContainerLifecycleState::Exited
                            | citadel_platforms::jobs::ContainerLifecycleState::Paused
                    ) {
                        drift.insert(stack);
                    }
                    if !swarm {
                        stacks.insert(stack);
                        if row.get::<String, _>("controlstate") == "Processing" {
                            degraded_stacks.insert(stack);
                        }
                    }
                }
            }
            // Task pruning is independent of Compose/Deployment lifecycle effects.
            prune |=
                swarm && delta.state == citadel_platforms::jobs::ContainerLifecycleState::Exited;
        } else {
            RuntimeWork::ContainerStateDeltaNoop.units(1);
        }
        results.push(ContainerDeltaResult {
            docker_id: delta.docker_id.clone(),
            accepted: true,
            observed: row.try_get("fresh")?,
            changed,
            container_id: Some(row.try_get("id")?),
            operation_id: row.try_get("containeroperationid")?,
            defer_parent_effects: owned,
            deployment_id: row.try_get("deploymentid")?,
            stack_id: row.try_get("stackid")?,
            patch: changed.then(|| ContainerStatePatch {
                id: row.get("id"),
                platform_id: platform,
                container_id: delta.docker_id.clone(),
                state: Some(
                    match delta.state {
                        citadel_platforms::jobs::ContainerLifecycleState::Running => "Running",
                        citadel_platforms::jobs::ContainerLifecycleState::Paused => "Paused",
                        citadel_platforms::jobs::ContainerLifecycleState::Exited => "Exited",
                    }
                    .to_owned(),
                ),
                // The command owns this field. Its completion can publish Idle
                // after coordination wakes but before this event is delivered.
                control_state: (!owned).then(|| row.get("controlstate")),
                updated: Some(delta.observed_at),
                docker_node_id: node.map(str::to_owned),
            }),
        });
    }
    // PostgreSQL notifications and parent writes share the state transaction.
    for id in drift {
        sqlx::query("SELECT pg_notify('citadel_stack_drift',$1)")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
    }
    if prune {
        sqlx::query("SELECT pg_notify('citadel_swarm_prune',$1)")
            .bind(platform.to_string())
            .execute(&mut **tx)
            .await?;
    }
    let normal: Vec<_> = stacks.difference(&degraded_stacks).copied().collect();
    for (ids, allow) in [
        (normal, false),
        (degraded_stacks.into_iter().collect(), true),
    ] {
        if !ids.is_empty() {
            let removed = RemovedBindings {
                affected_stacks: Some(ids),
                allow_degraded_while_processing: allow,
                ..Default::default()
            };
            reconcile(tx, platform, node, &removed, true).await?;
        }
    }
    Ok(results)
}

const STATE_DELTAS: &str = r#"
WITH incoming AS (
    SELECT * FROM unnest($3::text[], $4::text[], $5::bigint[], $6::uuid[])
        AS i(docker_id, state, observed, hint)
), current AS MATERIALIZED (
    SELECT c.id,c.dockercontainerid,c.deploymentid,c.stackid,c.isswarmtask,
           c.containeroperationid,c.controlstate,i.state AS next_state,i.observed,
           COALESCE(c.projectionobservedat,0) AS previous_observed,
           COALESCE(c.projectionobservedat,0) <= i.observed AS fresh,
           (lower(c.state) IS DISTINCT FROM i.state OR c.projectionstalesince IS NOT NULL
               OR c.projectionstalereason IS NOT NULL) AS semantic_change
    FROM incoming i JOIN containers c ON c.platformid=$1
        AND c.dockernodeid IS NOT DISTINCT FROM $2 AND c.dockercontainerid=i.docker_id
        AND (i.hint IS NULL OR c.id=i.hint)
    ORDER BY c.id FOR UPDATE OF c
), updated AS (
    UPDATE containers c SET state=initcap(r.next_state),
        updated=CASE WHEN r.semantic_change THEN r.observed ELSE c.updated END,
        projectionobservedat=GREATEST(COALESCE(c.projectionobservedat,0),r.observed),
        projectionstalesince=NULL, projectionstalereason=NULL,
        rowversion=c.rowversion + CASE WHEN r.semantic_change THEN 1 ELSE 0 END
    FROM current r WHERE c.id=r.id AND r.fresh
        AND (r.semantic_change OR r.previous_observed < r.observed)
    RETURNING c.id
)
SELECT *, (fresh AND semantic_change) AS changed FROM current
"#;
