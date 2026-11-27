use citadel_platforms::{
    ContainerStatSnapshot, MetricMembership, MetricWorkload, PlatformStatSnapshot,
    RuntimeCapabilityError, RuntimeErrorKind, ServiceStatIdentity, ServiceTaskSample,
    StatisticsContainer, StatisticsReader, StatisticsWorkload, StatsWindow, WorkloadStatSeries,
};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresStatisticsReader {
    pool: PgPool,
    history: std::sync::Arc<super::history_admission::Admission>,
}
impl PostgresStatisticsReader {
    pub fn new(pool: PgPool) -> Self {
        Self::with_tasks(
            pool,
            citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )
    }
    pub fn with_tasks(pool: PgPool, tasks: citadel_runtime::DynamicTasks) -> Self {
        let history = std::sync::Arc::new(super::history_admission::Admission::new(&pool, tasks));
        Self { pool, history }
    }
    pub fn with_admission(
        pool: PgPool,
        history: std::sync::Arc<super::history_admission::Admission>,
    ) -> Self {
        Self { pool, history }
    }
}
fn storage(e: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, e.to_string(), false)
}
fn map_container(row: PgRow) -> Result<ContainerStatSnapshot, RuntimeCapabilityError> {
    Ok(ContainerStatSnapshot {
        container_id: row.try_get("containerid").map_err(storage)?,
        block_read_bytes: row.try_get("blockreadbytes").map_err(storage)?,
        block_write_bytes: row.try_get("blockwritebytes").map_err(storage)?,
        block_read_bytes_delta: row.try_get("blockreadbytesdelta").map_err(storage)?,
        block_write_bytes_delta: row.try_get("blockwritebytesdelta").map_err(storage)?,
        rx_bytes_per_second: row.try_get("rxbytespersecond").map_err(storage)?,
        tx_bytes_per_second: row.try_get("txbytespersecond").map_err(storage)?,
        block_read_bytes_per_second: row.try_get("blockreadbytespersecond").map_err(storage)?,
        block_write_bytes_per_second: row.try_get("blockwritebytespersecond").map_err(storage)?,
        created: row.try_get("created").map_err(storage)?,
        cpu_usage: row.try_get("cpuusage").map_err(storage)?,
        memory_active: row.try_get("memoryactive").map_err(storage)?,
        memory_cache: row.try_get("memorycache").map_err(storage)?,
        memory_active_cache: row.try_get("memoryactivecache").map_err(storage)?,
        memory_limit: row.try_get("memorylimit").map_err(storage)?,
        rx_bytes: row.try_get("rxbytes").map_err(storage)?,
        tx_bytes: row.try_get("txbytes").map_err(storage)?,
        rx_bytes_delta: row.try_get("rxbytesdelta").map_err(storage)?,
        tx_bytes_delta: row.try_get("txbytesdelta").map_err(storage)?,
    })
}
impl StatisticsReader for PostgresStatisticsReader {
    fn metric_membership(
        &self,
        workload: MetricWorkload,
    ) -> BoxFuture<'_, Result<MetricMembership, RuntimeCapabilityError>> {
        const DEPLOYMENT: &str =
            "SELECT d.platformid, 1 AS expected, c.id, c.dockercontainerid, c.dockernodeid AS node
            FROM deployments d LEFT JOIN LATERAL (
                SELECT id, dockercontainerid, dockernodeid FROM containers
                WHERE deploymentid=d.id AND platformid=d.platformid AND projectionstalesince IS NULL
                ORDER BY updated DESC, id DESC LIMIT 1
            ) c ON true WHERE d.id=$1";
        const SERVICE: &str = concat!(
            "WITH scope AS (SELECT s.platformid,
                CASE WHEN p.platformdescriptor->>'$type'='DockerSwarm' THEN s.dockerserviceid END AS dockerserviceid
                FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE s.id=$1)",
            include_str!("service_task_members.sql"),
            "SELECT s.platformid, coalesce(p.desiredtaskcount,0) AS expected,
                CASE WHEN NOT p.isstale THEN m.id END AS id, m.dockercontainerid, m.node
             FROM scope s LEFT JOIN swarmserviceprojections p USING (platformid,dockerserviceid)
             LEFT JOIN members m ON true"
        );
        async move {
            let (id, sql) = match workload {
                MetricWorkload::Deployment(id) => (id, DEPLOYMENT),
                MetricWorkload::SwarmService(id) => (id, SERVICE),
            };
            let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            let mut membership = MetricMembership::default();
            if let Some(first) = rows.first() {
                membership.platform = first.try_get("platformid").map_err(storage)?;
                membership.expected =
                    first.try_get::<i32, _>("expected").map_err(storage)?.max(0) as usize;
            }
            // Partial, ambiguous or oversized membership falls back to bounded HTTP history.
            if rows.len() == membership.expected && rows.len() <= 500 {
                for row in rows {
                    let Some(id) = row.try_get::<Option<Uuid>, _>("id").map_err(storage)? else {
                        break;
                    };
                    let node = row.try_get("node").map_err(storage)?;
                    let docker_id = row.try_get("dockercontainerid").map_err(storage)?;
                    membership.members.insert((node, docker_id), id);
                }
                if membership.members.len() != membership.expected {
                    membership.members.clear();
                }
            }
            Ok(membership)
        }
        .boxed()
    }

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
            let rows = sqlx::query(concat!(
                "WITH scope AS (SELECT $1::uuid AS platformid,$2::text AS dockerserviceid)",
                include_str!("service_task_members.sql"),
                include_str!("service_current_tasks.sql")
            ))
            .bind(platform_id)
            .bind(service_id)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            if rows.len() > 500 {
                return Err(capacity(
                    "Service statistics supports at most 500 current Tasks",
                ));
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
    fn workload_history(
        &self,
        workload: StatisticsWorkload,
        id: Uuid,
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'_, Result<Option<Vec<WorkloadStatSeries>>, RuntimeCapabilityError>> {
        async move {
            let deployment = matches!(workload, StatisticsWorkload::Deployment);
            let exists = if deployment {
                "SELECT EXISTS(SELECT 1 FROM deployments WHERE id=$1)"
            } else {
                "SELECT EXISTS(SELECT 1 FROM stacks WHERE id=$1)"
            };
            if !sqlx::query_scalar::<_, bool>(exists).bind(id).fetch_one(&self.pool).await.map_err(storage)? {
                return Ok(None);
            }
            let mut read = self.history.begin(&self.pool).await?;
            let rows = sqlx::query(include_str!("workload_history.sql"))
                .bind(id).bind(deployment).bind(window.since(now)).bind(window.until(now))
                .bind(window.bucket_seconds_at(now)).fetch_all(read.connection()).await.map_err(super::history_admission::storage)?;
            if rows.len() > 65_536 {
                return Err(capacity("Workload history exceeds 65536 samples; choose a shorter range."));
            }
            let mut series = std::collections::BTreeMap::<String, WorkloadStatSeries>::new();
            for row in rows {
                let name: String = row.try_get("servicename").map_err(super::history_admission::storage)?;
                series.entry(name.clone()).or_insert_with(|| WorkloadStatSeries {
                    name, container_ids: vec![], stats: vec![],
                }).stats.push(map_container(row)?);
            }
            // Current membership is used only to merge live samples in the UI.
            // Historical identity comes from the owner recorded with each sample.
            let current = sqlx::query(r#"SELECT DISTINCT c.dockercontainerid,
                CASE WHEN $2 THEN '' ELSE COALESCE(s.name,NULLIF(c.composeservicename,''),c.id::text) END AS servicename
                FROM containers c
                LEFT JOIN swarmtaskprojections t ON c.isswarmtask AND t.platformid=c.platformid AND t.dockercontainerid=c.dockercontainerid
                    AND t.dockernodeid=COALESCE(c.dockernodeid,(SELECT platformdescriptor::jsonb->>'nodeID' FROM platforms WHERE id=c.platformid))
                LEFT JOIN swarmserviceprojections s ON s.platformid=t.platformid AND s.dockerserviceid=t.dockerserviceid
                WHERE ($2 AND c.deploymentid=$1) OR (NOT $2 AND (c.stackid=$1 OR s.stackid=$1))
                LIMIT 501"#).bind(id).bind(deployment).fetch_all(read.connection()).await.map_err(super::history_admission::storage)?;
            read.finish().await?;
            if current.len() > 500 {
                return Err(capacity("Statistics request exceeds 500 workload series or Containers"));
            }
            for row in current {
                let name: String = row.try_get("servicename").map_err(storage)?;
                series.entry(name.clone()).or_insert_with(|| WorkloadStatSeries {
                    name, container_ids: vec![], stats: vec![],
                }).container_ids.push(row.try_get("dockercontainerid").map_err(storage)?);
            }
            if series.len() > 500 {
                return Err(capacity("Statistics request exceeds 500 workload series or Containers"));
            }
            Ok(Some(series.into_values().collect()))
        }.boxed()
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
            let mut read = self.history.begin(&self.pool).await?;
            let rows = sqlx::query(include_str!("container_history.sql"))
                .bind(ids)
                .bind(window.since(now))
                .bind(window.bucket_seconds_at(now))
                .bind(window.until(now))
                .fetch_all(read.connection())
                .await
                .map_err(super::history_admission::storage)?;
            read.finish().await?;
            if rows.len() > 65_536 {
                return Err(capacity(
                    "History exceeds 65536 samples; request individual Container history.",
                ));
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
            let mut read = self.history.begin(&self.pool).await?;
            let rows = sqlx::query(include_str!("platform_history.sql"))
                .bind(id)
                .bind(window.since(now))
                .bind(window.bucket_seconds_at(now))
                .bind(window.until(now))
                .fetch_all(read.connection())
                .await
                .map_err(super::history_admission::storage)?;
            read.finish().await?;
            rows.into_iter()
                .map(|row| {
                    Ok(PlatformStatSnapshot {
                        container_io: None,
                        rx_bytes_delta: row.try_get("rxbytesdelta").map_err(storage)?,
                        tx_bytes_delta: row.try_get("txbytesdelta").map_err(storage)?,
                        block_read_bytes: row.try_get("blockreadbytes").map_err(storage)?,
                        block_write_bytes: row.try_get("blockwritebytes").map_err(storage)?,
                        block_read_bytes_delta: row
                            .try_get("blockreadbytesdelta")
                            .map_err(storage)?,
                        block_write_bytes_delta: row
                            .try_get("blockwritebytesdelta")
                            .map_err(storage)?,
                        rx_bytes_per_second: row.try_get("rxbytespersecond").map_err(storage)?,
                        tx_bytes_per_second: row.try_get("txbytespersecond").map_err(storage)?,
                        block_read_bytes_per_second: row
                            .try_get("blockreadbytespersecond")
                            .map_err(storage)?,
                        block_write_bytes_per_second: row
                            .try_get("blockwritebytespersecond")
                            .map_err(storage)?,

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
            let mut read = self.history.begin(&self.pool).await?;
            let rows = sqlx::query(include_str!("service_history.sql"))
                .bind(id.platform_id)
                .bind(id.docker_service_id)
                .bind(id.managed_service_id)
                .bind(id.stack_id)
                .bind(id.service_name)
                .bind(window.since(now))
                .bind(window.bucket_seconds_at(now))
                .bind(window.until(now))
                .fetch_all(read.connection())
                .await
                .map_err(super::history_admission::storage)?;
            read.finish().await?;
            if rows.len() > 65_536 {
                return Err(capacity(
                    "Service history exceeds 65536 samples; choose a shorter range.",
                ));
            }
            rows.into_iter().map(map_container).collect()
        }
        .boxed()
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
