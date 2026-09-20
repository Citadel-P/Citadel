use citadel_platforms::{
    ContainerStatSnapshot, PlatformStatSnapshot, RuntimeCapabilityError, RuntimeErrorKind,
    ServiceStatIdentity, ServiceTaskSample, StatisticsContainer, StatisticsReader,
    StatisticsWorkload, StatsWindow,
};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresStatisticsReader {
    pool: PgPool,
}
impl PostgresStatisticsReader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn storage(e: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, e.to_string(), false)
}
fn map_container(row: PgRow) -> Result<ContainerStatSnapshot, RuntimeCapabilityError> {
    Ok(ContainerStatSnapshot {
        container_id: row.try_get("containerid").map_err(storage)?,
        created: row.try_get("created").map_err(storage)?,
        cpu_usage: row.try_get("cpuusage").map_err(storage)?,
        memory_active: row.try_get("memoryactive").map_err(storage)?,
        memory_cache: row.try_get("memorycache").map_err(storage)?,
        memory_limit: row.try_get("memorylimit").map_err(storage)?,
        rx_bytes: row.try_get("rxbytes").map_err(storage)?,
        tx_bytes: row.try_get("txbytes").map_err(storage)?,
    })
}
impl StatisticsReader for PostgresStatisticsReader {
    fn task_container<'a>(
        &'a self,
        platform_id: Uuid,
        node_id: &'a str,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StatisticsContainer>, RuntimeCapabilityError>> {
        async move {
            let rows = sqlx::query(r#"SELECT container.id,container.platformid,container.dockercontainerid,container.name
                FROM containers container JOIN platforms platform ON platform.id=container.platformid
                WHERE container.platformid=$1 AND container.dockercontainerid=$3 AND container.projectionstalesince IS NULL
                AND COALESCE(container.dockernodeid,CASE WHEN platform.connectortype IN ('Local','Agent')
                    THEN platform.platformdescriptor::jsonb->>'nodeID' END)=$2 LIMIT 2"#)
                .bind(platform_id).bind(node_id).bind(container_id).fetch_all(&self.pool).await.map_err(storage)?;
            if rows.len() > 1 {
                return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Conflict,"Task Container projection is ambiguous.",false));
            }
            rows.into_iter().next().map(map_identity).transpose()
        }.boxed()
    }
    fn service_current_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: &'a str,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ServiceTaskSample>, RuntimeCapabilityError>> {
        async move {
            let rows=sqlx::query(r#"WITH current_tasks AS (
                SELECT DISTINCT ON (CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END)
                    dockernodeid,dockercontainerid
                FROM swarmtaskprojections WHERE platformid=$1 AND dockerserviceid=$2 AND NOT isstale
                    AND lower(state)='running' AND lower(desiredstate)='running'
                ORDER BY CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END,
                    COALESCE(statustimestamp,observedat) DESC,dockertaskid DESC LIMIT 501
            ) SELECT task.dockernodeid,container.id,latest.created
                FROM current_tasks task JOIN platforms platform ON platform.id=$1
                LEFT JOIN LATERAL (
                    SELECT CASE WHEN COUNT(*)=1 THEN (array_agg(candidate.id))[1] END AS id
                    FROM (SELECT id FROM containers
                        WHERE platformid=$1 AND task.dockernodeid=COALESCE(dockernodeid,
                            CASE WHEN platform.connectortype IN ('Local','Agent') THEN platform.platformdescriptor::jsonb->>'nodeID' END)
                        AND dockercontainerid=task.dockercontainerid AND projectionstalesince IS NULL LIMIT 2) candidate
                ) container ON true
                LEFT JOIN LATERAL (SELECT created FROM containerstats WHERE containerid=container.id AND created<=$3 ORDER BY created DESC LIMIT 1) latest ON true"#)
                .bind(platform_id).bind(service_id).bind(now).fetch_all(&self.pool).await.map_err(storage)?;
            if rows.len() > 500 {
                return Err(capacity("Service statistics supports at most 500 current Tasks"));
            }
            rows.into_iter()
                .map(|row| {
                    Ok(ServiceTaskSample {
                        node_id: row.try_get("dockernodeid").map_err(storage)?,
                        container_id: row.try_get("id").map_err(storage)?,
                        created: row.try_get("created").map_err(storage)?,
                    })
                })
                .collect()
        }
        .boxed()
    }
    fn find_container<'a>(
        &'a self,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<Option<StatisticsContainer>, RuntimeCapabilityError>> {
        async move {
            let rows = if let Ok(id) = Uuid::parse_str(reference) {
                sqlx::query("SELECT id,platformid,dockercontainerid,name FROM containers WHERE id=$1")
                    .bind(id).fetch_all(&self.pool).await
            } else {
                sqlx::query("SELECT id,platformid,dockercontainerid,name FROM containers WHERE starts_with(dockercontainerid,$1) LIMIT 2")
                    .bind(reference).fetch_all(&self.pool).await
            }.map_err(storage)?;
            if rows.len() > 1 {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Conflict,
                    "Container ID is ambiguous; use its projection ID",
                    false,
                ));
            }
            rows.into_iter().next().map(map_identity).transpose()
        }
        .boxed()
    }
    fn workload_containers(
        &self,
        workload: StatisticsWorkload,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<Vec<StatisticsContainer>>, RuntimeCapabilityError>> {
        async move {
            let (exists, query) = match workload {
                StatisticsWorkload::Deployment => (
                    "SELECT EXISTS(SELECT 1 FROM deployments WHERE id=$1)",
                    "SELECT id,platformid,dockercontainerid,name FROM containers WHERE deploymentid=$1 ORDER BY created DESC,id DESC LIMIT 1",
                ),
                StatisticsWorkload::Stack => (
                    "SELECT EXISTS(SELECT 1 FROM stacks WHERE id=$1)",
                    "SELECT id,platformid,dockercontainerid,name FROM containers WHERE stackid=$1 ORDER BY id LIMIT 501",
                ),
            };
            if !sqlx::query_scalar::<_, bool>(exists)
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)?
            {
                return Ok(None);
            }
            let rows = sqlx::query(query)
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            if rows.len() > 500 {
                return Err(capacity("Statistics request exceeds 500 Containers"));
            }
            Ok(Some(rows.into_iter().map(map_identity).collect::<Result<_, _>>()?))
        }
        .boxed()
    }
    fn containers<'a>(
        &'a self,
        ids: &'a [Uuid],
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ContainerStatSnapshot>, RuntimeCapabilityError>> {
        async move {
            if ids.is_empty() {
                return Ok(vec![]);
            }
            if ids.len() > 500 {
                return Err(capacity("Statistics request exceeds 500 Containers"));
            }
            let rows=sqlx::query(r#"SELECT containerid,MIN(created) AS created,COALESCE(AVG(cpuusage),0) AS cpuusage,
                COALESCE(AVG(memoryactive),0) AS memoryactive,COALESCE(AVG(memorycache),0) AS memorycache,
                COALESCE(AVG(memorylimit),0) AS memorylimit,COALESCE(AVG(rxbytes),0) AS rxbytes,COALESCE(AVG(txbytes),0) AS txbytes
                FROM containerstats WHERE containerid=ANY($1) AND created>$2 AND created<=$4
                GROUP BY containerid,(created/$3) ORDER BY MIN(created),containerid LIMIT 65537"#)
                .bind(ids).bind(window.since(now)).bind(window.bucket_seconds()).bind(now).fetch_all(&self.pool).await.map_err(storage)?;
            if rows.len() > 65_536 {
                return Err(capacity("History exceeds 65536 samples; request individual Container history."));
            }
            rows.into_iter().map(map_container).collect()
        }
        .boxed()
    }
    fn platform(
        &self,
        id: Uuid,
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'_, Result<Vec<PlatformStatSnapshot>, RuntimeCapabilityError>> {
        async move {
            let rows=sqlx::query(r#"SELECT MIN(created) AS created,AVG(cpuusage) AS cpuusage,AVG(memoryusage) AS memoryusage,
                AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,ROUND(AVG(diskusedbytes))::bigint AS diskusedbytes,
                ROUND(AVG(disktotalbytes))::bigint AS disktotalbytes,AVG(diskusage) AS diskusage
                FROM platformstats WHERE platformid=$1 AND created>$2 AND created<=$4 GROUP BY (created/$3) ORDER BY MIN(created)"#)
                .bind(id).bind(window.since(now)).bind(window.bucket_seconds()).bind(now).fetch_all(&self.pool).await.map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    Ok(PlatformStatSnapshot {
                        created: row.try_get("created").map_err(storage)?,
                        cpu_usage: row.try_get("cpuusage").map_err(storage)?,
                        memory_usage: row.try_get("memoryusage").map_err(storage)?,
                        rx_bytes: row.try_get("rxbytes").map_err(storage)?,
                        tx_bytes: row.try_get("txbytes").map_err(storage)?,
                        disk_used_bytes: row.try_get("diskusedbytes").map_err(storage)?,
                        disk_total_bytes: row.try_get("disktotalbytes").map_err(storage)?,
                        disk_usage: row.try_get("diskusage").map_err(storage)?,
                    })
                })
                .collect()
        }
        .boxed()
    }
    fn service<'a>(
        &'a self,
        id: ServiceStatIdentity<'a>,
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ContainerStatSnapshot>, RuntimeCapabilityError>> {
        async move {
            // Bucket by logical slot first: replacing a Task must not double count
            // replicas, and deleting Container projections must not erase history.
            sqlx::query(r#"WITH task_buckets AS (
                SELECT taskkey,(created/$7) AS bucket,MIN(created) AS created,AVG(cpuusage) AS cpuusage,
                    AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,AVG(memorylimit) AS memorylimit,
                    AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes FROM swarmservicestats
                WHERE created>$6 AND created<=$8 AND (
                    ($3::uuid IS NOT NULL AND (swarmserviceid=$3 OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
                    ($3 IS NULL AND $4::uuid IS NOT NULL AND ((stackid=$4 AND servicename=$5) OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
                    ($3 IS NULL AND $4 IS NULL AND platformid=$1 AND dockerserviceid=$2))
                GROUP BY taskkey,(created/$7))
                SELECT '00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
                    SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,SUM(memorycache) AS memorycache,
                    SUM(memorylimit) AS memorylimit,SUM(rxbytes) AS rxbytes,SUM(txbytes) AS txbytes
                FROM task_buckets GROUP BY bucket ORDER BY bucket"#)
                .bind(id.platform_id).bind(id.docker_service_id).bind(id.managed_service_id).bind(id.stack_id).bind(id.service_name)
                .bind(window.since(now)).bind(window.bucket_seconds()).bind(now).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(map_container).collect()
        }.boxed()
    }
}

fn map_identity(row: PgRow) -> Result<StatisticsContainer, RuntimeCapabilityError> {
    Ok(StatisticsContainer {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        docker_id: row
            .try_get::<Option<String>, _>("dockercontainerid")
            .map_err(storage)?
            .unwrap_or_default(),
        name: row.try_get("name").map_err(storage)?,
    })
}

fn capacity(message: &'static str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::ResourceExhausted, message, false)
}
