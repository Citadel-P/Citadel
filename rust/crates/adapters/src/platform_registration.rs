use chrono::Utc;
use citadel_domain::{ActivityEvent, ActivityEventInfo, ActorId, PlatformActivitySnapshot};
use citadel_platforms::{
    PlatformConnectorType, PlatformInventoryPort, PlatformRegistration, PlatformRegistrationError,
    PlatformRegistrationRuntime, PlatformRegistrationStore, RuntimeCapabilityError,
    RuntimeErrorKind,
};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;
use crate::agent::AgentClient;
use crate::docker::DockerClient;
use crate::inventory_projection_store::persist_snapshot;

#[derive(Clone)]
pub struct PlatformRegistrationRuntimeRouter {
    local: DockerClient,
    agent: Option<AgentClient>,
}

impl PlatformRegistrationRuntimeRouter {
    #[must_use]
    pub fn new(local: DockerClient, agent: Option<AgentClient>) -> Self {
        Self { local, agent }
    }
}

impl PlatformRegistrationRuntime for PlatformRegistrationRuntimeRouter {
    fn inventory_for<'a>(
        &'a self,
        connector_type: PlatformConnectorType,
        address: &'a str,
    ) -> Result<&'a dyn PlatformInventoryPort, PlatformRegistrationError> {
        match connector_type {
            PlatformConnectorType::Unknown => Err(PlatformRegistrationError::Runtime(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "A supported Platform connector type is required.",
                    false,
                ),
            )),
            PlatformConnectorType::Local => Ok(&self.local),
            PlatformConnectorType::Agent => self
                .agent
                .as_ref()
                .filter(|agent| same_address(agent.address(), address))
                .map(|agent| agent as &dyn PlatformInventoryPort)
                .ok_or_else(|| {
                    PlatformRegistrationError::Runtime(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "The requested Agent transport is not configured in the Rust Core.",
                        false,
                    ))
                }),
            PlatformConnectorType::EdgeAgent => Err(PlatformRegistrationError::Runtime(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    "The Edge Agent is disconnected or unavailable.",
                    true,
                ),
            )),
        }
    }
}

fn same_address(left: &str, right: &str) -> bool {
    left.trim_end_matches('/')
        .eq_ignore_ascii_case(right.trim_end_matches('/'))
}

#[derive(Clone)]
pub struct PostgresPlatformRegistrationStore {
    pool: PgPool,
}

impl PostgresPlatformRegistrationStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PlatformRegistrationStore for PostgresPlatformRegistrationStore {
    fn create_pending_edge<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        input: &'a citadel_platforms::CreatePlatformInput,
    ) -> BoxFuture<'a, Result<Uuid, PlatformRegistrationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM platforms WHERE lower(name)=lower($1))",
            )
            .bind(&input.name)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if exists {
                return Err(PlatformRegistrationError::Conflict(
                    "A Platform with this name already exists.".into(),
                ));
            }
            validate_tags(&mut tx, &input.tag_ids).await?;
            let address = format!("edge://{id}");
            let descriptor = serde_json::json!({"$type": if input.platform_type == citadel_platforms::PlatformType::DockerSwarm { "DockerSwarm" } else { "DockerStandalone" }, "daemonId": ""});
            sqlx::query("INSERT INTO platforms(id,name,address,description,connectortype,cpucount,imagecount,memtotal,networkcount,platformdescriptor,status,volumecount,prunehistoricalswarmtaskcontainers) VALUES($1,$2,$3,$4,'EdgeAgent',0,0,0,0,$5,'Offline',0,$6)")
                .bind(id).bind(&input.name).bind(&address).bind(&input.description).bind(&descriptor).bind(input.prune_historical_swarm_task_containers).execute(&mut *tx).await.map_err(database_error)?;
            crate::resource_tags::insert(&mut tx, "Platform", id, &input.tag_ids, actor_id.value())
                .await
                .map_err(|error| match error {
                    crate::resource_tags::ResourceTagError::Missing => {
                        PlatformRegistrationError::Validation(
                            "One or more Tags do not exist.".into(),
                        )
                    }
                    crate::resource_tags::ResourceTagError::Database(error) => storage(error),
                })?;
            let event = ActivityEvent::new_platform_event(
                id,
                input.name.clone(),
                actor_id,
                ActivityEventInfo::platform_created(PlatformActivitySnapshot {
                    id,
                    name: input.name.clone(),
                    address,
                    description: input.description.clone(),
                    status: "Offline".into(),
                    connector_type: "EdgeAgent".into(),
                    network_count: 0,
                    volume_count: 0,
                    image_count: 0,
                    cpu_count: 0,
                    mem_total: 0,
                    server_version: None,
                    agent_version: None,
                    platform_descriptor: descriptor,
                }),
                Utc::now(),
            )
            .map_err(|error| PlatformRegistrationError::Storage(error.to_string()))?;
            insert_activity(&mut tx, &event)
                .await
                .map_err(|error| PlatformRegistrationError::Storage(error.to_string()))?;
            tx.commit().await.map_err(storage)?;
            Ok(id)
        })
    }
    fn ensure_available<'a>(
        &'a self,
        name: &'a str,
        address: &'a str,
    ) -> BoxFuture<'a, Result<(), PlatformRegistrationError>> {
        async move {
            let existing = sqlx::query_scalar::<_, String>(
                "SELECT name FROM platforms WHERE lower(name) = lower($1) OR address = $2 LIMIT 1",
            )
            .bind(name)
            .bind(address)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            if existing.is_some() {
                return Err(PlatformRegistrationError::Conflict(
                    "A platform with the same name or address already exists.".to_owned(),
                ));
            }
            Ok(())
        }
        .boxed()
    }

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        registration: &'a PlatformRegistration,
    ) -> BoxFuture<'a, Result<Uuid, PlatformRegistrationError>> {
        async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            // Platform registration is rare. One transaction-scoped lock closes
            // name and Docker identity races that are not all protected by the
            // legacy schema's indexes.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            reject_existing(&mut transaction, registration).await?;
            validate_tags(&mut transaction, &registration.tag_ids).await?;
            insert_platform(&mut transaction, registration).await?;
            persist_snapshot(&mut transaction, &registration.snapshot)
                .await
                .map_err(|error| PlatformRegistrationError::Storage(error.message))?;
            insert_tags(&mut transaction, actor_id, registration).await?;
            insert_created_activity(&mut transaction, actor_id, registration).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(registration.id)
        }
        .boxed()
    }
}

async fn reject_existing(
    transaction: &mut Transaction<'_, Postgres>,
    registration: &PlatformRegistration,
) -> Result<(), PlatformRegistrationError> {
    let existing = sqlx::query(
        r#"
SELECT name,
       COALESCE(platformdescriptor::jsonb->>'daemonId' = $3, FALSE) AS same_daemon,
       COALESCE($4::text IS NOT NULL AND clusterid = $4, FALSE) AS same_cluster
FROM platforms
WHERE lower(name) = lower($1)
   OR address = $2
   OR platformdescriptor::jsonb->>'daemonId' = $3
   OR ($4::text IS NOT NULL AND clusterid = $4)
ORDER BY name
LIMIT 1
FOR UPDATE
"#,
    )
    .bind(&registration.name)
    .bind(&registration.address)
    .bind(registration.snapshot.info.daemon_id.trim())
    .bind(&registration.cluster_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?;
    let Some(existing) = existing else {
        return Ok(());
    };
    let name: String = existing.try_get("name").map_err(storage)?;
    if existing
        .try_get::<bool, _>("same_daemon")
        .map_err(storage)?
    {
        return Err(PlatformRegistrationError::Conflict(format!(
            "This Docker engine is already registered as platform '{name}'."
        )));
    }
    if existing
        .try_get::<bool, _>("same_cluster")
        .map_err(storage)?
    {
        return Err(PlatformRegistrationError::Conflict(format!(
            "This Swarm cluster is already registered as platform '{name}'."
        )));
    }
    Err(PlatformRegistrationError::Conflict(
        "A platform with the same name or address already exists.".to_owned(),
    ))
}

async fn validate_tags(
    transaction: &mut Transaction<'_, Postgres>,
    tag_ids: &[Uuid],
) -> Result<(), PlatformRegistrationError> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE id = ANY($1)")
        .bind(tag_ids)
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)?;
    if usize::try_from(count).ok() != Some(tag_ids.len()) {
        return Err(PlatformRegistrationError::Validation(
            "One or more Tags do not exist.".to_owned(),
        ));
    }
    Ok(())
}

async fn insert_platform(
    transaction: &mut Transaction<'_, Postgres>,
    registration: &PlatformRegistration,
) -> Result<(), PlatformRegistrationError> {
    sqlx::query(
        r#"
INSERT INTO platforms (
    id, address, agentversion, clusterid, connectortype, cpucount, description,
    imagecount, memtotal, name, networkcount, platformdescriptor,
    prunehistoricalswarmtaskcontainers, serverversion, status, volumecount
)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'Online',$15)
"#,
    )
    .bind(registration.id)
    .bind(&registration.address)
    .bind(&registration.snapshot.info.agent_version)
    .bind(&registration.cluster_id)
    .bind(registration.connector_type.as_str())
    .bind(bounded_i32(registration.snapshot.info.cpu_count))
    .bind(&registration.description)
    .bind(bounded_i32(registration.snapshot.images.len()))
    .bind(registration.snapshot.info.memory_total)
    .bind(&registration.name)
    .bind(bounded_i32(registration.snapshot.networks.len()))
    .bind(&registration.descriptor)
    .bind(registration.prune_historical_swarm_task_containers)
    .bind(&registration.snapshot.info.server_version)
    .bind(bounded_i32(registration.snapshot.volumes.len()))
    .execute(&mut **transaction)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn insert_tags(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    registration: &PlatformRegistration,
) -> Result<(), PlatformRegistrationError> {
    for tag_id in &registration.tag_ids {
        sqlx::query(
            "INSERT INTO resourcetags (resourcetype,resourceid,tagid,createdbyactorid) VALUES ('Platform',$1,$2,$3)",
        )
        .bind(registration.id)
        .bind(tag_id)
        .bind(actor_id.value())
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    }
    Ok(())
}

async fn insert_created_activity(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    registration: &PlatformRegistration,
) -> Result<(), PlatformRegistrationError> {
    let snapshot = &registration.snapshot;
    let event = ActivityEvent::new_platform_event(
        registration.id,
        registration.name.clone(),
        actor_id,
        ActivityEventInfo::platform_created(PlatformActivitySnapshot {
            id: registration.id,
            name: registration.name.clone(),
            address: registration.address.clone(),
            description: registration.description.clone(),
            status: "Online".to_owned(),
            connector_type: registration.connector_type.as_str().to_owned(),
            network_count: bounded_i32(snapshot.networks.len()),
            volume_count: bounded_i32(snapshot.volumes.len()),
            image_count: bounded_i64(snapshot.images.len()),
            cpu_count: snapshot.info.cpu_count,
            mem_total: snapshot.info.memory_total,
            server_version: Some(snapshot.info.server_version.clone()),
            agent_version: snapshot.info.agent_version.clone(),
            platform_descriptor: registration.descriptor.clone(),
        }),
        Utc::now(),
    )
    .map_err(|error| PlatformRegistrationError::Storage(error.to_string()))?;
    insert_activity(transaction, &event)
        .await
        .map_err(|error| PlatformRegistrationError::Storage(error.to_string()))
}

fn bounded_i32(value: impl TryInto<i32>) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}

fn bounded_i64(value: impl TryInto<i64>) -> i64 {
    value.try_into().unwrap_or(i64::MAX)
}

fn storage(error: sqlx::Error) -> PlatformRegistrationError {
    PlatformRegistrationError::Storage(error.to_string())
}

fn database_error(error: sqlx::Error) -> PlatformRegistrationError {
    if let sqlx::Error::Database(database) = &error {
        match database.constraint() {
            Some("ix_platforms_address") => {
                return PlatformRegistrationError::Conflict(
                    "A platform with the same address already exists.".to_owned(),
                );
            }
            Some("ix_platforms_clusterid") => {
                return PlatformRegistrationError::Conflict(
                    "This Swarm cluster is already registered.".to_owned(),
                );
            }
            _ => {}
        }
    }
    storage(error)
}
