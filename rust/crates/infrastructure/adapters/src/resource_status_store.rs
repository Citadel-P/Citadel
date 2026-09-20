//! Shared event and reconciliation rules, matching the .NET container work items.
use citadel_activities::{ActivityEvent, ActivityEventInfo, ActivityStatus};
use citadel_primitives::ActorId;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

type Result<T> = std::result::Result<T, sqlx::Error>;
#[derive(Default)]
pub(crate) struct RemovedBindings {
    deployments: Vec<Uuid>,
    affected_deployments: Option<Vec<Uuid>>,
    affected_stacks: Option<Vec<Uuid>>,
    stacks: Vec<Uuid>,
    allow_degraded_while_processing: bool,
}

pub(crate) async fn removed_bindings(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    incoming: &[String],
    observed: i64,
) -> Result<RemovedBindings> {
    let rows = sqlx::query("SELECT deploymentid,stackid,isswarmtask FROM containers WHERE platformid=$1 AND dockernodeid IS NOT DISTINCT FROM $2 AND NOT(dockercontainerid=ANY($3)) AND COALESCE(projectionobservedat,0)<=$4")
        .bind(platform).bind(node).bind(incoming).bind(observed).fetch_all(&mut **tx).await?;
    let mut removed = RemovedBindings::default();
    for row in rows {
        if let Some(id) = row.try_get::<Option<Uuid>, _>("deploymentid")? {
            removed.deployments.push(id);
        }
        if !row.try_get::<bool, _>("isswarmtask")?
            && let Some(id) = row.try_get::<Option<Uuid>, _>("stackid")?
        {
            removed.stacks.push(id);
        }
    }
    Ok(removed)
}

/// Apply an observed state to one persisted runtime identity. No full inventory scan.
/// `None` represents a confirmed daemon deletion, never a failed inspection.
pub async fn container_event(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    docker_id: &str,
    state: Option<&str>,
    name: Option<&str>,
    observed: i64,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let changed =
        container_event_in(&mut tx, platform, node, docker_id, state, name, observed).await?;
    tx.commit().await?;
    Ok(changed)
}
pub(crate) async fn container_event_in(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    docker_id: &str,
    state: Option<&str>,
    name: Option<&str>,
    observed: i64,
) -> Result<bool> {
    // Use the same lock order as inventory and health synchronization.
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(platform)
        .fetch_optional(&mut **tx)
        .await?;
    let row=sqlx::query("SELECT id,deploymentid,stackid,isswarmtask,controlstate FROM containers WHERE platformid=$1 AND dockernodeid IS NOT DISTINCT FROM $2 AND dockercontainerid=$3 AND COALESCE(projectionobservedat,0)<=$4 FOR UPDATE")
        .bind(platform).bind(node).bind(docker_id).bind(observed).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(false) };
    let id: Uuid = row.try_get("id")?;
    let mut removed = RemovedBindings {
        allow_degraded_while_processing: row.try_get::<String, _>("controlstate")? == "Processing",
        affected_deployments: Some(
            row.try_get::<Option<Uuid>, _>("deploymentid")?
                .into_iter()
                .collect(),
        ),
        affected_stacks: Some(if row.try_get::<bool, _>("isswarmtask")? {
            vec![]
        } else {
            row.try_get::<Option<Uuid>, _>("stackid")?
                .into_iter()
                .collect()
        }),
        ..Default::default()
    };
    if let Some(state) = state {
        sqlx::query("UPDATE containers SET state=initcap($2),name=COALESCE($3,name),updated=$4,projectionobservedat=$4,rowversion=rowversion+1 WHERE id=$1")
            .bind(id).bind(state).bind(name.map(|s|s.trim_start_matches('/'))).bind(observed).execute(&mut **tx).await?;
    } else {
        removed
            .deployments
            .extend(row.try_get::<Option<Uuid>, _>("deploymentid")?);
        if !row.try_get::<bool, _>("isswarmtask")? {
            removed
                .stacks
                .extend(row.try_get::<Option<Uuid>, _>("stackid")?);
        }
        sqlx::query("DELETE FROM containers WHERE id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    if state.is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "exited" | "paused"))
        && let Some(stack) = row.try_get::<Option<Uuid>, _>("stackid")?
    {
        sqlx::query("SELECT pg_notify('citadel_stack_drift', $1)")
            .bind(stack.to_string())
            .execute(&mut **tx)
            .await?;
    }
    if row.try_get::<bool, _>("isswarmtask")?
        && state.is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "exited" | "dead"))
    {
        sqlx::query("SELECT pg_notify('citadel_swarm_prune', $1)")
            .bind(platform.to_string())
            .execute(&mut **tx)
            .await?;
    }
    reconcile(tx, platform, node, &removed, true).await?;
    Ok(true)
}

pub async fn platform_online(pool: &PgPool, platform: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    platform_status(&mut tx, platform, "Online").await?;
    tx.commit().await
}

pub async fn platform_offline(pool: &PgPool, platform: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    platform_offline_in(&mut tx, platform).await?;
    tx.commit().await
}
pub(crate) async fn platform_offline_in(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
) -> Result<()> {
    platform_status(tx, platform, "Offline").await?;
    // Match DeploymentSyncJob: never turn a never-deployed resource into Degraded.
    sqlx::query("UPDATE deployments SET status='Degraded',rowversion=rowversion+1 WHERE platformid=$1 AND status NOT IN ('Created','Degraded','Applying') AND controlstate='Idle'")
        .bind(platform).execute(&mut **tx).await?;
    sqlx::query("UPDATE containers SET state='Offline',rowversion=rowversion+1 WHERE platformid=$1 AND dockernodeid IS NULL AND state<>'Offline'")
        .bind(platform).execute(&mut **tx).await?;
    reconcile(tx, platform, None, &RemovedBindings::default(), false).await?;
    Ok(())
}

pub(crate) async fn reconcile(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    removed: &RemovedBindings,
    activities: bool,
) -> Result<()> {
    sqlx::query("SAVEPOINT deployment_sync")
        .execute(&mut **tx)
        .await?;
    if let Err(error) = reconcile_deployments(tx, platform, node, removed, activities).await {
        sqlx::query("ROLLBACK TO SAVEPOINT deployment_sync")
            .execute(&mut **tx)
            .await?;
        tracing::warn!(%error,%platform,"Deployment synchronization failed; continuing Stack synchronization");
    }
    sqlx::query("RELEASE SAVEPOINT deployment_sync")
        .execute(&mut **tx)
        .await?;
    sqlx::query("SAVEPOINT stack_sync")
        .execute(&mut **tx)
        .await?;
    if let Err(error) = reconcile_stacks(tx, platform, node, removed, activities).await {
        sqlx::query("ROLLBACK TO SAVEPOINT stack_sync")
            .execute(&mut **tx)
            .await?;
        tracing::warn!(%error,%platform,"Stack synchronization failed; periodic reconciliation will retry");
    }
    sqlx::query("RELEASE SAVEPOINT stack_sync")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn reconcile_deployments(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    removed: &RemovedBindings,
    activities: bool,
) -> Result<()> {
    let deployments=sqlx::query("SELECT d.id,d.name,d.status,d.controlstate,d.controltriggeredby,c.state,c.dockercontainerid FROM deployments d LEFT JOIN LATERAL (SELECT state,dockercontainerid FROM containers WHERE deploymentid=d.id ORDER BY updated DESC,id DESC LIMIT 1) c ON TRUE WHERE d.platformid=$1 AND (EXISTS(SELECT 1 FROM containers WHERE deploymentid=d.id AND dockernodeid IS NOT DISTINCT FROM $2) OR d.id=ANY($3) OR ($2::text IS NULL AND c.dockercontainerid IS NULL)) AND ($4::uuid[] IS NULL OR d.id=ANY($4)) ORDER BY d.id FOR UPDATE OF d")
        .bind(platform).bind(node).bind(&removed.deployments).bind(&removed.affected_deployments).fetch_all(&mut **tx).await?;
    for row in deployments {
        let old: String = row.try_get("status")?;
        if row.try_get::<String, _>("controlstate")? != "Idle" || old == "Applying" {
            continue;
        }
        let id: Uuid = row.try_get("id")?;
        let state: Option<String> = row.try_get("state")?;
        let next = if removed.deployments.contains(&id) {
            "Degraded"
        } else if let Some(state) = state {
            deployment_status(&state)
        } else if old == "Applying" || (!activities && old == "Created") {
            continue;
        } else {
            "Degraded"
        };
        if next == old {
            continue;
        }
        sqlx::query("UPDATE deployments SET status=$2,rowversion=rowversion+1 WHERE id=$1")
            .bind(id)
            .bind(next)
            .execute(&mut **tx)
            .await?;
        if activities {
            let ids = row
                .try_get::<Option<String>, _>("dockercontainerid")?
                .into_iter()
                .collect();
            let info = match next {
                "Healthy" => Some(ActivityEventInfo::DeploymentStarted { container_ids: ids }),
                "Stopped" => Some(ActivityEventInfo::DeploymentStopped { container_ids: ids }),
                "Pending" => Some(ActivityEventInfo::DeploymentPaused { container_ids: ids }),
                "Degraded" => Some(ActivityEventInfo::DeploymentDegraded {
                    reason: "The associated container is missing or unavailable.".into(),
                }),
                _ => None,
            };
            if let Some(info) = info {
                activity(tx, &row, platform, info, next == "Degraded", false).await?;
            }
        }
    }
    Ok(())
}

async fn reconcile_stacks(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    removed: &RemovedBindings,
    activities: bool,
) -> Result<()> {
    // Swarm service/task health is owned by Swarm reconciliation, not Compose rules.
    let stacks=sqlx::query("SELECT s.id,s.name,s.controlstate,s.controltriggeredby,r.id releaseid,r.status,p.status platformstatus FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE r.platformid=$1 AND lower(COALESCE(p.platformdescriptor->>'$type','')) <> 'dockerswarm' AND (EXISTS(SELECT 1 FROM containers c WHERE c.stackid=s.id AND NOT c.isswarmtask AND c.dockernodeid IS NOT DISTINCT FROM $2) OR s.id=ANY($3) OR $2::text IS NULL) AND ($4::uuid[] IS NULL OR s.id=ANY($4)) ORDER BY s.id FOR UPDATE OF s,r")
        .bind(platform).bind(node).bind(&removed.stacks).bind(&removed.affected_stacks).fetch_all(&mut **tx).await?;
    for row in stacks {
        let old: String = row.try_get("status")?;
        if old == "Applying" || (!activities && old == "Created") {
            continue;
        }
        let id: Uuid = row.try_get("id")?;
        if !activities && row.try_get::<String, _>("controlstate")? == "Processing" {
            if !matches!(old.as_str(), "Applying" | "Pending") {
                sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1")
                    .bind(id).execute(&mut **tx).await?;
            }
            continue;
        }
        let containers:Vec<(String,String)>=sqlx::query_as("SELECT dockercontainerid,lower(state) FROM containers WHERE stackid=$1 AND NOT isswarmtask ORDER BY dockercontainerid")
            .bind(id).fetch_all(&mut **tx).await?;
        let next = if row.try_get::<String, _>("platformstatus")? == "Offline"
            || removed.stacks.contains(&id)
        {
            "Degraded"
        } else {
            stack_status(
                &containers
                    .iter()
                    .map(|(_, s)| s.as_str())
                    .collect::<Vec<_>>(),
            )
        };
        if old == next
            || (activities
                && row.try_get::<String, _>("controlstate")? == "Processing"
                && (next == "Pending"
                    || (next == "Degraded" && !removed.allow_degraded_while_processing)))
        {
            continue;
        }
        sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=$1")
            .bind(row.try_get::<Uuid, _>("releaseid")?)
            .bind(next)
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE stacks SET rowversion=rowversion+1,controlstate=CASE WHEN $2 THEN 'Idle' ELSE controlstate END,controlstartedat=CASE WHEN $2 THEN NULL ELSE controlstartedat END,controltriggeredby=CASE WHEN $2 THEN NULL ELSE controltriggeredby END WHERE id=$1")
            .bind(id).bind(activities)
            .execute(&mut **tx)
            .await?;
        if activities {
            let ids = containers.into_iter().map(|(id, _)| id).collect();
            let info = match next {
                "Healthy" => Some(ActivityEventInfo::StackStarted { container_ids: ids }),
                "Stopped" => Some(ActivityEventInfo::StackStopped { container_ids: ids }),
                "Paused" => Some(ActivityEventInfo::StackPaused { container_ids: ids }),
                "Degraded" => Some(ActivityEventInfo::StackDegraded {
                    reason:
                        "One or more associated containers are missing or not running normally."
                            .into(),
                }),
                _ => None,
            };
            if let Some(info) = info {
                activity(tx, &row, platform, info, next == "Degraded", true).await?;
            }
        }
    }
    Ok(())
}
async fn activity(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
    platform: Uuid,
    info: ActivityEventInfo,
    warning: bool,
    stack: bool,
) -> Result<()> {
    let actor = row
        .try_get::<Option<Uuid>, _>("controltriggeredby")?
        .unwrap_or(citadel_identity::SYSTEM_ACTOR_ID);
    let constructor = if stack {
        ActivityEvent::new_stack_event
    } else {
        ActivityEvent::new_deployment_result_event
    };
    let event = constructor(
        row.try_get("id")?,
        row.try_get("name")?,
        platform,
        ActorId::new(actor),
        info,
        if warning {
            ActivityStatus::Warning
        } else {
            ActivityStatus::Success
        },
        chrono::Utc::now(),
    )
    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))
}
fn deployment_status(state: &str) -> &'static str {
    match state.to_ascii_lowercase().as_str() {
        "running" => "Healthy",
        "exited" => "Stopped",
        "paused" | "restarting" => "Pending",
        "created" => "Created",
        "dead" | "offline" => "Degraded",
        _ => "Failed",
    }
}
fn stack_status(states: &[&str]) -> &'static str {
    if states.is_empty() || states.contains(&"offline") {
        "Degraded"
    } else if states.iter().all(|s| *s == "running") {
        "Healthy"
    } else if states.iter().all(|s| *s == "paused") {
        "Paused"
    } else if states.iter().all(|s| matches!(*s, "exited" | "offline")) {
        "Stopped"
    } else if states
        .iter()
        .any(|s| matches!(*s, "created" | "restarting" | "removing"))
    {
        "Pending"
    } else if states.iter().all(|s| *s == "dead") {
        "Failed"
    } else if states.iter().all(|s| *s == "unknown") {
        "Unknown"
    } else {
        "Degraded"
    }
}

/// Persist exactly one connection activity with the confirmed transition.
pub(crate) async fn platform_status(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    status: &str,
) -> Result<()> {
    use citadel_activities::PlatformActivitySnapshot;
    let row = sqlx::query("SELECT id,name,address,description,status,connectortype,networkcount,volumecount,imagecount::bigint imagecount,cpucount::bigint cpucount,memtotal,serverversion,agentversion,platformdescriptor FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(id).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let previous_status: String = row.try_get("status")?;
    if previous_status == status {
        return Ok(());
    }
    sqlx::query("UPDATE platforms SET status=$2 WHERE id=$1")
        .bind(id)
        .bind(status)
        .execute(&mut **tx)
        .await?;
    let platform = PlatformActivitySnapshot {
        id,
        name: row.try_get("name")?,
        address: row.try_get("address")?,
        description: row.try_get("description")?,
        status: status.into(),
        connector_type: row.try_get("connectortype")?,
        network_count: row.try_get("networkcount")?,
        volume_count: row.try_get("volumecount")?,
        image_count: row.try_get("imagecount")?,
        cpu_count: row.try_get("cpucount")?,
        mem_total: row.try_get("memtotal")?,
        server_version: row.try_get("serverversion")?,
        agent_version: row.try_get("agentversion")?,
        platform_descriptor: row.try_get("platformdescriptor")?,
    };
    let name = platform.name.clone();
    let info = if status == "Online" {
        ActivityEventInfo::PlatformConnected {
            platform,
            previous_status,
        }
    } else {
        ActivityEventInfo::PlatformDisconnected {
            platform,
            previous_status,
        }
    };
    let event = ActivityEvent::new_platform_event(
        id,
        name,
        ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
        info,
        chrono::Utc::now(),
    )
    .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    crate::activity_store::insert_activity(tx, &event)
        .await
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))
}

pub async fn container_observation(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    container: &citadel_platforms::RuntimeContainerSummary,
    observed: i64,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    container_metadata_in(&mut tx, platform, node, container, observed).await?;
    let changed = container_event_in(
        &mut tx,
        platform,
        node,
        &container.id,
        Some(&container.state),
        Some(&container.name),
        observed,
    )
    .await?;
    tx.commit().await?;
    Ok(changed)
}

pub(crate) async fn container_metadata_in(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    container: &citadel_platforms::RuntimeContainerSummary,
    observed: i64,
) -> Result<()> {
    crate::inventory_projection_store::persist_container_observation(
        tx, platform, node, container, observed,
    )
    .await
    .map_err(|error| sqlx::Error::Protocol(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stack_states_follow_dotnet_precedence() {
        for (states, expected) in [
            (vec![], "Degraded"),
            (vec!["running", "running"], "Healthy"),
            (vec!["paused", "paused"], "Paused"),
            (vec!["exited", "offline"], "Degraded"),
            (vec!["exited"], "Stopped"),
            (vec!["offline"], "Degraded"),
            (vec!["running", "restarting"], "Pending"),
            (vec!["dead", "created"], "Pending"),
            (vec!["removing"], "Pending"),
            (vec!["dead", "dead"], "Failed"),
            (vec!["unknown"], "Unknown"),
            (vec!["running", "exited"], "Degraded"),
        ] {
            assert_eq!(stack_status(&states), expected);
        }
    }
    #[test]
    fn status_activities_keep_dotnet_wire_names() {
        for (info, kind) in [
            (
                ActivityEventInfo::DeploymentStarted {
                    container_ids: vec!["container".into()],
                },
                "DeploymentStarted",
            ),
            (
                ActivityEventInfo::DeploymentStopped {
                    container_ids: vec!["container".into()],
                },
                "DeploymentStopped",
            ),
            (
                ActivityEventInfo::DeploymentPaused {
                    container_ids: vec!["container".into()],
                },
                "DeploymentPaused",
            ),
            (
                ActivityEventInfo::DeploymentDegraded {
                    reason: "missing".into(),
                },
                "DeploymentDegraded",
            ),
            (
                ActivityEventInfo::StackDegraded {
                    reason: "missing".into(),
                },
                "StackDegraded",
            ),
        ] {
            let json = serde_json::to_value(&info).unwrap();
            assert_eq!(json["$type"], kind);
            assert!(json.get("ContainerIds").is_some() || json.get("Reason").is_some());
        }
    }
}
