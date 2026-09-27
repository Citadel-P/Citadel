//! Parent status, lifecycle activities and claim release are one idempotent commit.
use super::super::status;
use citadel_activities::ActivityEventInfo;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind,
    containers::{ContainerClaim, ContainerCompletion, ContainerStatePatch},
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(super) async fn finish(
    pool: &PgPool,
    claim: Uuid,
    selection: Option<&ContainerClaim>,
) -> Result<ContainerCompletion, RuntimeCapabilityError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    if let Some(selection) =
        selection.filter(|s| s.deployment_ids.is_empty() && s.stack_ids.is_empty())
    {
        let platforms: Vec<_> = selection
            .targets
            .iter()
            .map(|target| target.platform_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        sqlx::query("SELECT id FROM platforms WHERE id=ANY($1) ORDER BY id FOR SHARE")
            .bind(platforms)
            .fetch_all(&mut *tx)
            .await
            .map_err(storage)?;
    } else {
        // Parent claims can survive deletion of their last target. Resolve
        // their platforms from durable ownership and retain the lock order.
        super::repository::lock_claim_resources(&mut tx, claim).await?;
    }
    let deployments = if selection.is_some_and(|s| s.deployment_ids.is_empty()) {
        vec![]
    } else {
        sqlx::query("SELECT id,name,status,platformid,controltriggeredby FROM deployments WHERE containeroperationid=$1 AND ($2::uuid[] IS NULL OR id=ANY($2)) ORDER BY id FOR NO KEY UPDATE")
            .bind(claim).bind(selection.map(|s| &s.deployment_ids)).fetch_all(&mut *tx).await.map_err(storage)?
    };
    let stacks = if selection.is_some_and(|s| s.stack_ids.is_empty()) {
        vec![]
    } else {
        sqlx::query("SELECT s.id,s.name,s.controltriggeredby,r.id releaseid,r.status,r.platformid FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.containeroperationid=$1 AND ($2::uuid[] IS NULL OR s.id=ANY($2)) ORDER BY s.id FOR NO KEY UPDATE OF s,r")
            .bind(claim).bind(selection.map(|s| &s.stack_ids)).fetch_all(&mut *tx).await.map_err(storage)?
    };
    let deployment_ids: Vec<Uuid> = deployments.iter().map(|r| r.get("id")).collect();
    let stack_ids: Vec<Uuid> = stacks.iter().map(|r| r.get("id")).collect();
    // Include unselected siblings when deriving a parent's final status. Match
    // event reconciliation's preference for the latest surviving Deployment row.
    let containers = if deployment_ids.is_empty() && stack_ids.is_empty() {
        vec![]
    } else {
        sqlx::query("SELECT dockercontainerid,lower(state) state,deploymentid,stackid FROM containers WHERE NOT isswarmtask AND (deploymentid=ANY($1) OR stackid=ANY($2)) ORDER BY updated DESC,id DESC")
        .bind(&deployment_ids).bind(&stack_ids).fetch_all(&mut *tx).await.map_err(storage)?
    };
    for row in &deployments {
        let id: Uuid = row.get("id");
        let container = containers
            .iter()
            .find(|c| c.get::<Option<Uuid>, _>("deploymentid") == Some(id));
        let next = container.map_or("Degraded", |c| status::deployment_status(c.get("state")));
        if row.get::<String, _>("status") == next {
            continue;
        }
        sqlx::query("UPDATE deployments SET status=$2 WHERE id=$1 AND containeroperationid=$3")
            .bind(id)
            .bind(next)
            .bind(claim)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let ids = container
            .map(|c| c.get::<String, _>("dockercontainerid"))
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
            status::activity(
                &mut tx,
                row,
                row.get("platformid"),
                info,
                next == "Degraded",
                false,
            )
            .await
            .map_err(storage)?;
        }
    }
    for row in &stacks {
        let id: Uuid = row.get("id");
        let members: Vec<_> = containers
            .iter()
            .filter(|c| c.get::<Option<Uuid>, _>("stackid") == Some(id))
            .collect();
        let states: Vec<&str> = members.iter().map(|c| c.get("state")).collect();
        let next = status::stack_status(&states);
        if row.get::<String, _>("status") == next {
            continue;
        }
        sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=$1")
            .bind(row.get::<Uuid, _>("releaseid"))
            .bind(next)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let mut ids: Vec<String> = members
            .iter()
            .filter(|c| match next {
                "Healthy" => c.get::<&str, _>("state") == "running",
                "Paused" => c.get::<&str, _>("state") == "paused",
                "Stopped" => matches!(c.get::<&str, _>("state"), "exited" | "offline"),
                _ => false,
            })
            .map(|c| c.get("dockercontainerid"))
            .collect();
        ids.sort();
        ids.dedup();
        let info = match next {
            "Healthy" => Some(ActivityEventInfo::StackStarted { container_ids: ids }),
            "Stopped" => Some(ActivityEventInfo::StackStopped { container_ids: ids }),
            "Paused" => Some(ActivityEventInfo::StackPaused { container_ids: ids }),
            "Degraded" => Some(ActivityEventInfo::StackDegraded {
                reason: "One or more associated containers are missing or not running normally."
                    .into(),
            }),
            _ => None,
        };
        if let Some(info) = info {
            status::activity(
                &mut tx,
                row,
                row.get("platformid"),
                info,
                next == "Degraded",
                true,
            )
            .await
            .map_err(storage)?;
        }
    }
    let mut completion = ContainerCompletion::default();
    for (table, parents) in [("deployments", &deployment_ids), ("stacks", &stack_ids)] {
        if parents.is_empty() {
            continue;
        }
        let statement = format!(
            "UPDATE {table} SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,containeroperationid=NULL,rowversion=rowversion+1 WHERE containeroperationid=$1 RETURNING id"
        );
        let ids: Vec<Uuid> = sqlx::query_scalar(sqlx::AssertSqlSafe(statement))
            .bind(claim)
            .fetch_all(&mut *tx)
            .await
            .map_err(storage)?;
        match table {
            "deployments" => completion.deployment_ids = ids,
            "stacks" => completion.stack_ids = ids,
            _ => unreachable!(),
        }
    }
    completion.container_patches = sqlx::query(
        "UPDATE containers SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,containeroperationid=NULL,rowversion=rowversion+1 WHERE containeroperationid=$1 RETURNING id,platformid,dockercontainerid,state,controlstate,updated,dockernodeid",
    )
    .bind(claim)
    .fetch_all(&mut *tx)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| ContainerStatePatch {
        id: row.get("id"),
        platform_id: row.get("platformid"),
        container_id: row.get("dockercontainerid"),
        state: Some(row.get("state")),
        control_state: Some(row.get("controlstate")),
        updated: row.get("updated"),
        docker_node_id: row.get("dockernodeid"),
    })
    .collect();
    tx.commit().await.map_err(storage)?;
    Ok(completion)
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}
