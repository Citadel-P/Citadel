use citadel_stacks::{StackError, StackRuntimeService, StackRuntimeSnapshot};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) async fn swarm_snapshot(
    pool: &PgPool,
    platform_id: Uuid,
    project: &str,
) -> Result<StackRuntimeSnapshot, StackError> {
    let rows = sqlx::query("SELECT dockerserviceid,name,versionindex,desiredtaskcount,runningtaskcount,updatestate,updatemessage FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY dockerserviceid")
                    .bind(platform_id).bind(project).fetch_all(pool).await.map_err(storage)?;
    Ok(StackRuntimeSnapshot {
        containers: Vec::new(),
        services: rows
            .into_iter()
            .map(|row| {
                Ok(StackRuntimeService {
                    docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                    version_index: row.try_get("versionindex").map_err(storage)?,
                    desired_tasks: row.try_get("desiredtaskcount").map_err(storage)?,
                    running_tasks: row.try_get("runningtaskcount").map_err(storage)?,
                    update_state: row.try_get("updatestate").map_err(storage)?,
                    update_message: row.try_get("updatemessage").map_err(storage)?,
                })
            })
            .collect::<Result<_, StackError>>()?,
    })
}

pub(crate) async fn compose_container_ids(
    pool: &PgPool,
    platform_id: Uuid,
    project: &str,
) -> Result<Vec<String>, StackError> {
    sqlx::query_scalar(
            "SELECT dockercontainerid FROM containers WHERE platformid=$1 AND stack=$2 AND NOT isswarmtask ORDER BY dockercontainerid",
        )
        .bind(platform_id)
        .bind(project)
        .fetch_all(pool)
        .await
        .map_err(storage)
}

pub(crate) async fn swarm_service_ids(
    pool: &PgPool,
    platform_id: Uuid,
    project: &str,
) -> Result<Vec<String>, StackError> {
    sqlx::query_scalar(
            "SELECT dockerserviceid FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY dockerserviceid",
        )
        .bind(platform_id)
        .bind(project)
        .fetch_all(pool)
        .await
        .map_err(storage)
}

pub(crate) async fn import_platform(
    pool: &PgPool,
    platform_id: Uuid,
) -> Result<(String, Value), StackError> {
    let row = sqlx::query(
        "SELECT name,platformdescriptor FROM platforms WHERE id=$1 AND status='Online'",
    )
    .bind(platform_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?
    .ok_or(StackError::NotFound)?;
    Ok((
        row.try_get("name").map_err(storage)?,
        row.try_get("platformdescriptor").map_err(storage)?,
    ))
}

pub(crate) struct ImportService {
    pub name: String,
    pub image: String,
    pub desired_tasks: i32,
    pub running_tasks: i32,
    pub update_state: String,
    pub labels: Value,
    pub ownership: String,
    pub stack_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
}
pub(crate) async fn import_services(
    pool: &PgPool,
    platform_id: Uuid,
    project_name: &str,
) -> Result<Vec<ImportService>, StackError> {
    let rows = sqlx::query("SELECT dockerserviceid,name,image,desiredtaskcount,runningtaskcount,updatestate,labels,ownership,stackid,swarmserviceid FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY name")
                    .bind(platform_id).bind(project_name).fetch_all(pool).await.map_err(storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(ImportService {
                name: row.try_get("name").map_err(storage)?,
                image: row.try_get("image").map_err(storage)?,
                desired_tasks: row.try_get("desiredtaskcount").map_err(storage)?,
                running_tasks: row.try_get("runningtaskcount").map_err(storage)?,
                update_state: row.try_get("updatestate").map_err(storage)?,
                labels: row.try_get("labels").map_err(storage)?,
                ownership: row.try_get("ownership").map_err(storage)?,
                stack_id: row.try_get("stackid").map_err(storage)?,
                service_id: row.try_get("swarmserviceid").map_err(storage)?,
            })
        })
        .collect()
}

pub(crate) async fn stack_exists(pool: &PgPool, id: Uuid) -> Result<bool, StackError> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM stacks WHERE id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}

pub(crate) async fn validate_import_containers(
    pool: &PgPool,
    platform_id: Uuid,
    project: &str,
    expected: std::collections::BTreeSet<String>,
) -> Result<(), StackError> {
    let rows = sqlx::query("SELECT dockercontainerid,issystem,isswarmtask,deploymentid,stackid,controlstate FROM containers WHERE platformid=$1 AND stack=$2")
            .bind(platform_id).bind(project).fetch_all(pool).await.map_err(storage)?;
    let persisted = rows
        .iter()
        .map(|r| r.try_get::<String, _>("dockercontainerid").map_err(storage))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    if persisted != expected {
        return Err(StackError::Conflict(
            "Compose project membership changed. Refresh the Platform and retry.".into(),
        ));
    }
    for row in &rows {
        if row.try_get::<bool, _>("issystem").map_err(storage)?
            || row.try_get::<bool, _>("isswarmtask").map_err(storage)?
            || row
                .try_get::<Option<Uuid>, _>("deploymentid")
                .map_err(storage)?
                .is_some()
            || row
                .try_get::<Option<Uuid>, _>("stackid")
                .map_err(storage)?
                .is_some()
            || row
                .try_get::<Option<String>, _>("controlstate")
                .map_err(storage)?
                .as_deref()
                == Some("Processing")
        {
            return Err(StackError::Conflict(
                "Compose project contains system, managed or processing containers.".into(),
            ));
        }
    }
    Ok(())
}

fn storage(error: sqlx::Error) -> StackError {
    StackError::Storage(error.to_string())
}
