use std::collections::BTreeSet;

use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, containers::*};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresContainerRepository {
    pool: PgPool,
    coordinator: std::sync::Arc<ContainerOperationCoordinator>,
    authorization: std::sync::Arc<
        crate::persistence::postgres::identity::authorization_cache::AuthorizationCache,
    >,
}

impl PostgresContainerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            coordinator: super::coordination::attach(&pool),
            authorization: crate::persistence::postgres::identity::authorization_cache::AuthorizationCache::attach(&pool),
            pool,
        }
    }
}

impl ContainerRepository for PostgresContainerRepository {
    fn coordinator(&self) -> Option<std::sync::Arc<ContainerOperationCoordinator>> {
        Some(self.coordinator.clone())
    }
    fn finish_committed(
        &self,
        claim: Uuid,
    ) -> BoxFuture<'_, Result<ContainerCompletion, RuntimeCapabilityError>> {
        Box::pin(super::completion::finish(&self.pool, claim))
    }

    fn resolve_ids<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Uuid>, RuntimeCapabilityError>> {
        Box::pin(async move {
            if let Ok(uuids) = ids
                .iter()
                .map(|id| Uuid::parse_str(id))
                .collect::<Result<Vec<_>, _>>()
            {
                return Ok(uuids);
            }
            let mut resolved = Vec::with_capacity(ids.len());
            for id in ids {
                if let Ok(id) = Uuid::parse_str(id) {
                    resolved.push(id);
                    continue;
                }
                if let Some(index) = super::super::runtime_index::attached(&self.pool) {
                    match index.lookup_prefix(id) {
                        Some(super::super::runtime_index::PrefixMatch::Unique(id)) => {
                            resolved.push(id);
                            continue;
                        }
                        Some(super::super::runtime_index::PrefixMatch::Ambiguous) => {
                            return Err(error(
                                RuntimeErrorKind::Conflict,
                                "Container ID is ambiguous across nodes; use its Citadel ID.",
                            ));
                        }
                        None => {}
                    }
                }
                let prefix = id.to_ascii_lowercase();
                let _query =
                    citadel_runtime::runtime_metrics::RuntimeWork::ContainerIdResolveQuery.start();
                let matches: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM containers WHERE left(lower(dockercontainerid),length($1))=$1 ORDER BY id LIMIT 2")
                    .bind(prefix).fetch_all(&self.pool).await.map_err(storage)?;
                match matches.as_slice() {
                    [id] => resolved.push(*id),
                    [] => {
                        return Err(error(
                            RuntimeErrorKind::NotFound,
                            "No containers found for the provided ID(s).",
                        ));
                    }
                    _ => {
                        return Err(error(
                            RuntimeErrorKind::Conflict,
                            "Container ID is ambiguous across nodes; use its Citadel ID.",
                        ));
                    }
                }
            }
            Ok(resolved)
        })
    }
    fn claim_selection<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
        action: ContainerAction,
        selection: ContainerSelectionKind,
    ) -> BoxFuture<'a, Result<ContainerClaim, RuntimeCapabilityError>> {
        Box::pin(async move {
            // Fence ACL mutations before opening a SQL transaction. Cold cache
            // fills then use the claim transaction below; warm admissions avoid
            // the actor/team/role CTE entirely.
            let authorization = if administrator {
                None
            } else {
                let fence = self.authorization.read_fence().await;
                Some(fence)
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Inventory takes an exclusive Platform lock before its child rows.
            // Take shared Platform locks first, including for Deployment selectors.
            let platforms = match selection {
                ContainerSelectionKind::Containers => {
                    "SELECT p.id FROM platforms p WHERE p.id IN (SELECT platformid FROM containers WHERE id=ANY($1::uuid[])) ORDER BY p.id FOR SHARE OF p"
                }
                ContainerSelectionKind::Deployments => {
                    "SELECT p.id FROM platforms p WHERE p.id IN (SELECT platformid FROM deployments WHERE id=ANY($1::uuid[])) ORDER BY p.id FOR SHARE OF p"
                }
            };
            sqlx::query(platforms)
                .bind(ids)
                .fetch_all(&mut *tx)
                .await
                .map_err(storage)?;
            let selected_deployments = if selection == ContainerSelectionKind::Deployments {
                ids
            } else {
                &[]
            };
            let container_ids;
            let ids = if selection == ContainerSelectionKind::Deployments {
                let deployments = sqlx::query("SELECT d.id,p.platformdescriptor FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id=ANY($1) ORDER BY d.id FOR UPDATE OF d")
                    .bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
                if deployments.len() != ids.len() {
                    return Err(error(
                        RuntimeErrorKind::NotFound,
                        "No deployments found for the provided deployment ID(s).",
                    ));
                }
                if deployments.iter().any(|row| {
                    row.get::<serde_json::Value, _>("platformdescriptor")["$type"] == "DockerSwarm"
                }) {
                    return Err(error(
                        RuntimeErrorKind::InvalidRequest,
                        "Container state actions are not available for Docker Swarm deployments.",
                    ));
                }
                let matches: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT c.id,c.deploymentid FROM containers c JOIN deployments d ON d.id=c.deploymentid AND d.platformid=c.platformid WHERE d.id=ANY($1) AND NOT c.isswarmtask ORDER BY c.id LIMIT $2")
                    .bind(ids).bind(ids.len() as i64 + 1).fetch_all(&mut *tx).await.map_err(storage)?;
                if matches.len() != ids.len()
                    || matches
                        .iter()
                        .map(|(_, id)| id)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != ids.len()
                {
                    return Err(error(
                        RuntimeErrorKind::Conflict,
                        "Each selected Deployment must have exactly one current Container.",
                    ));
                }
                container_ids = matches.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
                container_ids.as_slice()
            } else {
                ids
            };
            // Every writer locks parents before children. Stable ordering also avoids inverse batch locks.
            let parents = sqlx::query("SELECT deploymentid,stackid FROM containers WHERE id=ANY($1::uuid[]) AND NOT isswarmtask")
                .bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
            let deployments: BTreeSet<Uuid> = parents
                .iter()
                .filter_map(|r| r.get("deploymentid"))
                .collect();
            let stacks: BTreeSet<Uuid> = parents.iter().filter_map(|r| r.get("stackid")).collect();
            for (table, parent_ids) in [("deployments", &deployments), ("stacks", &stacks)] {
                if parent_ids.is_empty() {
                    continue;
                }
                let statement = format!(
                    "SELECT id,controlstate FROM {table} WHERE id=ANY($1::uuid[]) ORDER BY id FOR UPDATE"
                );
                let rows = sqlx::query(sqlx::AssertSqlSafe(statement))
                    .bind(parent_ids.iter().copied().collect::<Vec<_>>())
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(storage)?;
                if rows.len() != parent_ids.len()
                    || rows.iter().any(|r| {
                        r.get::<Option<String>, _>("controlstate").as_deref() != Some("Idle")
                    })
                {
                    return Err(conflict());
                }
            }
            let rows = sqlx::query("SELECT c.id,c.platformid,c.dockercontainerid,c.dockernodeid,c.deploymentid,c.stackid,c.controlstate,c.issystem,c.isswarmtask,c.state,c.projectionstalesince,p.status platformstatus FROM containers c JOIN platforms p ON p.id=c.platformid WHERE c.id=ANY($1::uuid[]) ORDER BY c.id FOR UPDATE OF c")
                .bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
            if rows.len() != ids.len() {
                return Err(error(
                    RuntimeErrorKind::NotFound,
                    "No containers found for the provided ID(s).",
                ));
            }
            let platform_ids: Vec<Uuid> = rows
                .iter()
                .map(|r| r.get("platformid"))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if !administrator {
                let (resource_type, resource_ids) =
                    if selection == ContainerSelectionKind::Deployments {
                        (ResourceType::Deployment, selected_deployments)
                    } else {
                        (ResourceType::Platform, platform_ids.as_slice())
                    };
                let fence = authorization
                    .as_ref()
                    .expect("non-administrator claim holds an authorization fence");
                let grants = self
                    .authorization
                    .resources_in_transaction(&mut tx, actor, resource_type, resource_ids, fence)
                    .await
                    .map_err(storage)?;
                let required = if selection == ContainerSelectionKind::Deployments {
                    PermissionLevel::Write as i32
                } else if matches!(action, ContainerAction::Delete(_)) {
                    PermissionLevel::Execute as i32
                } else {
                    PermissionLevel::Write as i32 | PermissionLevel::Execute as i32
                };
                if resource_ids.iter().any(|id| {
                    grants
                        .get(id)
                        .and_then(|grant| *grant)
                        .map_or(0, |grant| grant.level as i32)
                        & required
                        == 0
                }) {
                    return Err(error(
                        RuntimeErrorKind::PermissionDenied,
                        "Not authorized to operate on all selected Containers.",
                    ));
                }
            }
            for row in &rows {
                if selection == ContainerSelectionKind::Deployments
                    && !row
                        .get::<Option<Uuid>, _>("deploymentid")
                        .is_some_and(|id| selected_deployments.contains(&id))
                {
                    return Err(conflict());
                }
                if row.get::<Option<String>, _>("controlstate").as_deref() != Some("Idle") {
                    return Err(conflict());
                }
                if row.get::<bool, _>("issystem") {
                    return Err(error(
                        RuntimeErrorKind::Conflict,
                        "Citadel system Containers cannot be changed here.",
                    ));
                }
                if row.get::<bool, _>("isswarmtask") {
                    let stopped = matches!(
                        row.get::<String, _>("state").to_ascii_lowercase().as_str(),
                        "exited" | "dead"
                    );
                    if !matches!(action, ContainerAction::Delete(_)) || !stopped {
                        return Err(error(
                            RuntimeErrorKind::Conflict,
                            "Manage active Swarm Tasks through their Service.",
                        ));
                    }
                }
                if row.get::<String, _>("platformstatus") != "Online"
                    || row.get::<Option<i64>, _>("projectionstalesince").is_some()
                {
                    return Err(error(
                        RuntimeErrorKind::Unavailable,
                        "Container inventory is stale or its Platform is unavailable.",
                    ));
                }
                // Adoption may have changed the parent between the discovery and row locks.
                if !row.get::<bool, _>("isswarmtask")
                    && (row
                        .get::<Option<Uuid>, _>("deploymentid")
                        .is_some_and(|id| !deployments.contains(&id))
                        || row
                            .get::<Option<Uuid>, _>("stackid")
                            .is_some_and(|id| !stacks.contains(&id)))
                {
                    return Err(conflict());
                }
            }
            let operation_id = Uuid::now_v7();
            let deployment_ids = deployments.iter().copied().collect();
            let stack_ids = stacks.iter().copied().collect();
            for (table, targets) in [
                ("deployments", deployments.into_iter().collect::<Vec<_>>()),
                ("stacks", stacks.into_iter().collect()),
                ("containers", ids.to_vec()),
            ] {
                if targets.is_empty() {
                    continue;
                }
                let statement = format!(
                    "UPDATE {table} SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,containeroperationid=$4,rowversion=rowversion+1 WHERE id=ANY($1::uuid[])"
                );
                sqlx::query(sqlx::AssertSqlSafe(statement))
                    .bind(targets)
                    .bind(chrono::Utc::now().timestamp())
                    .bind(actor.value())
                    .bind(operation_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            let targets = rows.into_iter().map(target).collect();
            sqlx::query("SELECT pg_notify($1, '')")
                .bind(citadel_runtime::RuntimeSignal::ContainerRecovery.channel())
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(ContainerClaim {
                operation_id,
                started_at: chrono::Utc::now().timestamp(),
                targets,
                deployment_ids,
                stack_ids,
            })
        })
    }

    fn observed<'a>(
        &'a self,
        claim: Uuid,
        target: &'a ContainerTarget,
        state: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.observed_batch(
                claim,
                &[ContainerObservation {
                    target: target.clone(),
                    state: state.map(str::to_owned),
                }],
            )
            .await
        })
    }

    fn observed_batch<'a>(
        &'a self,
        claim: Uuid,
        observations: &'a [ContainerObservation],
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(super::observations::persist(
            &self.pool,
            claim,
            observations,
        ))
    }

    fn finish(&self, claim: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move { self.finish_committed(claim).await.map(|_| ()) })
    }

    fn abandon(&self, claim: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_claim_resources(&mut tx, claim).await?;
            sqlx::query("UPDATE deployments SET status='Unknown' WHERE containeroperationid=$1")
                .bind(claim)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("UPDATE stackreleases r SET status='Unknown' FROM stacks s WHERE s.containeroperationid=$1 AND r.id=s.currentstackreleaseid").bind(claim).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE containers SET projectionstalesince=COALESCE(projectionstalesince,$2),projectionstalereason='Container operation could not be verified on its owning node.' WHERE containeroperationid=$1").bind(claim).bind(chrono::Utc::now().timestamp()).execute(&mut *tx).await.map_err(storage)?;
            for table in ["deployments", "stacks", "containers"] {
                let statement = format!(
                    "UPDATE {table} SET controlstate='Idle',containeroperationid=NULL,controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE containeroperationid=$1"
                );
                sqlx::query(sqlx::AssertSqlSafe(statement))
                    .bind(claim)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn stale(&self) -> BoxFuture<'_, Result<Vec<ContainerClaim>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let _query =
                citadel_runtime::runtime_metrics::RuntimeWork::ContainerRecoveryQuery.start();
            let ids: Vec<(Uuid,i64)> = sqlx::query_as("SELECT operationid,min(controlstartedat) FROM (SELECT containeroperationid operationid,controlstartedat FROM containers UNION ALL SELECT containeroperationid,controlstartedat FROM deployments UNION ALL SELECT containeroperationid,controlstartedat FROM stacks) claims WHERE operationid IS NOT NULL AND controlstartedat<$1 GROUP BY operationid ORDER BY min(controlstartedat),operationid LIMIT 10")
                .bind(chrono::Utc::now().timestamp()-60).fetch_all(&self.pool).await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for (operation_id, started_at) in ids {
                let targets = sqlx::query("SELECT id,platformid,dockercontainerid,dockernodeid FROM containers WHERE containeroperationid=$1 ORDER BY id")
                    .bind(operation_id).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(target).collect();
                claims.push(ContainerClaim {
                    operation_id,
                    started_at,
                    targets,
                    deployment_ids: sqlx::query_scalar(
                        "SELECT id FROM deployments WHERE containeroperationid=$1",
                    )
                    .bind(operation_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(storage)?,
                    stack_ids: sqlx::query_scalar(
                        "SELECT id FROM stacks WHERE containeroperationid=$1",
                    )
                    .bind(operation_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(storage)?,
                });
            }
            citadel_runtime::runtime_metrics::RuntimeWork::ContainerRecoveryQuery
                .units(claims.len() as u64);
            Ok(claims)
        })
    }
}

fn target(row: sqlx::postgres::PgRow) -> ContainerTarget {
    ContainerTarget {
        id: row.get("id"),
        platform_id: row.get("platformid"),
        docker_id: row.get("dockercontainerid"),
        node_id: row.get("dockernodeid"),
    }
}
pub(super) async fn lock_claim_resources(
    tx: &mut Transaction<'_, Postgres>,
    claim: Uuid,
) -> Result<(), RuntimeCapabilityError> {
    // Include parents: deletion may have already removed every claimed container.
    // Stable ordering also protects operations spanning multiple Platforms.
    sqlx::query(
        "SELECT p.id FROM platforms p WHERE p.id IN (
            SELECT platformid FROM containers WHERE containeroperationid=$1
            UNION SELECT platformid FROM deployments WHERE containeroperationid=$1
            UNION SELECT r.platformid FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.containeroperationid=$1
        ) ORDER BY p.id FOR SHARE OF p",
    )
    .bind(claim)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    // Match claim acquisition and Stack operations: parents before release or
    // container rows, including when finalizing an uncertain runtime outcome.
    for table in ["deployments", "stacks"] {
        let statement = format!(
            "SELECT id FROM {table} WHERE containeroperationid=$1 ORDER BY id FOR NO KEY UPDATE"
        );
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(claim)
            .fetch_all(&mut **tx)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

fn conflict() -> RuntimeCapabilityError {
    error(
        RuntimeErrorKind::Conflict,
        "A selected Container or its parent has an operation in progress.",
    )
}
fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}
fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}
