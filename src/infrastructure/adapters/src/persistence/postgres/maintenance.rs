//! Maintenance runs independently of dispatch being enabled and of new statistics writes.
use sqlx::PgPool;

// Keep the indexed bigint column uncast. EXTRACT returns numeric, which otherwise
// makes PostgreSQL filter the retained history instead of using an index bound.
// CEIL preserves `integer timestamp < fractional cutoff`, including whole seconds.
const STATS_RETENTION_SQL: [&str; 4] = [
    "DELETE FROM workloadstats WHERE id IN(SELECT id FROM workloadstats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
    "DELETE FROM containerstats WHERE id IN(SELECT id FROM containerstats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
    "DELETE FROM platformstats WHERE id IN(SELECT id FROM platformstats WHERE NOT alertpending AND created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
    "DELETE FROM swarmservicestats WHERE id IN(SELECT id FROM swarmservicestats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)",
];

pub async fn reconcile(pool: &PgPool) -> Result<Vec<&'static str>, sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::Recovery.start();
    let mut changed = Vec::new();
    let mut tx = pool.begin().await?;
    // Existing recovery predicates protect live runs until their execution deadlines.
    let before = chrono::Utc::now();
    if !crate::persistence::postgres::automation::recover_interrupted(&mut tx, before)
        .await
        .map_err(protocol)?
        .is_empty()
    {
        changed.extend(["AutomationAction"]);
    }
    if crate::persistence::postgres::builds::recover(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend(["BuildProject", "BuildRun"]);
    }
    if crate::persistence::postgres::backups::recover_stale(&mut tx, before)
        .await
        .map_err(protocol)?
        > 0
    {
        changed.extend([
            "BackupRepository",
            "BackupPolicy",
            "BackupRun",
            "BackupRestoreRun",
        ]);
    }
    for (resource, sql) in [
        (
            "Deployment",
            "UPDATE deployments SET status='Unknown',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600)::bigint AND containeroperationid IS NULL",
        ),
        (
            "Stack",
            "WITH stuck AS (UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600)::bigint AND containeroperationid IS NULL RETURNING currentstackreleaseid) UPDATE stackreleases SET status='Unknown' WHERE id IN(SELECT currentstackreleaseid FROM stuck)",
        ),
        (
            "Container",
            "UPDATE containers SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND containeroperationid IS NULL AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-60)::bigint",
        ),
        (
            "GitRepository",
            "UPDATE gitrepositories SET status='Degraded',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-3600)::bigint",
        ),
        (
            "Image",
            "UPDATE images SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-60)::bigint",
        ),
        (
            "AutomationAction",
            "UPDATE actions a SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND NOT EXISTS(SELECT 1 FROM automationexecutorreservations held WHERE held.runid=a.currentrunid) AND NOT EXISTS(SELECT 1 FROM actionruns r WHERE r.id=a.currentrunid AND r.status IN('Queued','Running'))",
        ),
        (
            "BuildProject",
            "UPDATE buildprojects p SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND NOT EXISTS(SELECT 1 FROM buildruns r WHERE r.id=p.currentrunid AND r.status IN('Queued','Preparing','Running'))",
        ),
        (
            "BuildAgentPool",
            "UPDATE buildagentpools SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND controlstartedat<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-300)::bigint",
        ),
    ] {
        if sqlx::query(sql).execute(&mut *tx).await?.rows_affected() > 0 {
            changed.push(resource);
        }
    }
    tx.commit().await?;
    if let Err(error) = observe_execution_capacity(pool).await {
        tracing::warn!(%error, "Execution capacity metrics could not be sampled");
    }
    changed.sort_unstable();
    changed.dedup();
    Ok(changed)
}

pub async fn cleanup(pool: &PgPool, build_retention_days: Option<i32>) -> Result<u64, sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::Retention.start();
    // Retention must run even when every runtime is offline; it is not inventory work.
    let mut deleted = sqlx::query("DELETE FROM containerhealthtransitions WHERE id IN (SELECT id FROM containerhealthtransitions WHERE recordedat < now() - interval '7 days' ORDER BY recordedat LIMIT 1000 FOR UPDATE SKIP LOCKED)")
        .execute(pool).await?.rows_affected();
    citadel_runtime::runtime_metrics::RuntimeWork::ContainerHealthRetention.units(deleted);
    // Bound each transaction. Repeated scheduled batches drain backlogs while keeping
    // locks short and retaining every active run and referenced build artifact.
    for sql in STATS_RETENTION_SQL.into_iter().chain([
        "DELETE FROM activityevents WHERE id IN(SELECT id FROM activityevents WHERE createdat<CURRENT_TIMESTAMP-INTERVAL '90 days' LIMIT 5000)",
        "DELETE FROM actionruns WHERE id IN(SELECT r.id FROM actionruns r WHERE finishedat<CURRENT_TIMESTAMP-INTERVAL '90 days' AND status NOT IN('Queued','Running') AND NOT EXISTS(SELECT 1 FROM actions a WHERE a.currentrunid=r.id) LIMIT 5000)",
    ]) {
        deleted += sqlx::query(sql).execute(pool).await?.rows_affected();
    }
    if let Some(days) = build_retention_days.filter(|days| *days > 0) {
        deleted += sqlx::query("DELETE FROM buildruns b WHERE b.id IN(SELECT r.id FROM buildruns r WHERE r.completedat<CURRENT_TIMESTAMP-make_interval(days=>$1) AND r.status IN('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND NOT EXISTS(SELECT 1 FROM buildexecutorreservations reservation WHERE reservation.runid=r.id) AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=r.id) AND NOT EXISTS(SELECT 1 FROM buildprojects p WHERE p.currentrunid=r.id) AND NOT EXISTS(SELECT 1 FROM deployments d WHERE jsonb_path_exists(d.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) AND NOT EXISTS(SELECT 1 FROM stackreleases s WHERE jsonb_path_exists(s.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) ORDER BY r.completedat LIMIT 1000)")
            .bind(days).execute(pool).await?.rows_affected();
    }
    citadel_runtime::runtime_metrics::RuntimeWork::Retention.units(deleted);
    Ok(deleted)
}
/// Refresh tokens have a separate expiry cadence. Backup leases are ownership
/// evidence: only verified cleanup may release them, regardless of expiry.
pub async fn expire_leases(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::LeaseExpiry.start();
    let count = sqlx::query("DELETE FROM refreshtokens WHERE id IN(SELECT id FROM refreshtokens WHERE expiresat<CURRENT_TIMESTAMP ORDER BY expiresat LIMIT 5000)")
        .execute(pool).await?.rows_affected();
    citadel_runtime::runtime_metrics::RuntimeWork::LeaseExpiry.units(count);
    Ok(())
}

fn protocol(error: impl std::fmt::Display) -> sqlx::Error {
    sqlx::Error::Protocol(error.to_string())
}

/// Only call while holding the exclusive Core job lease, before dispatch starts.
pub async fn recover_on_startup(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let before = chrono::Utc::now();
    crate::persistence::postgres::automation::recover_interrupted_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::persistence::postgres::backups::recover_stale_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    crate::persistence::postgres::builds::recover_with_mode(&mut tx, before, true)
        .await
        .map_err(protocol)?;
    tx.commit().await
}

pub struct PostgresMaintenanceStore(pub PgPool);
impl citadel_runtime::maintenance::MaintenanceStore for PostgresMaintenanceStore {
    fn reconcile(
        &self,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<Vec<&'static str>, citadel_runtime::maintenance::MaintenanceError>,
    > {
        Box::pin(async move {
            reconcile(&self.0)
                .await
                .map_err(|e| citadel_runtime::maintenance::MaintenanceError(e.to_string()))
        })
    }
    fn expire_leases(
        &self,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<(), citadel_runtime::maintenance::MaintenanceError>,
    > {
        Box::pin(async move {
            expire_leases(&self.0)
                .await
                .map(|_| ())
                .map_err(|e| citadel_runtime::maintenance::MaintenanceError(e.to_string()))
        })
    }
    fn cleanup(
        &self,
        days: Option<i32>,
    ) -> futures_util::future::BoxFuture<
        '_,
        Result<u64, citadel_runtime::maintenance::MaintenanceError>,
    > {
        Box::pin(async move {
            cleanup(&self.0, days)
                .await
                .map_err(|e| citadel_runtime::maintenance::MaintenanceError(e.to_string()))
        })
    }
}

// Fixed families and aggregate values only; no run, host or repository labels.
async fn observe_execution_capacity(pool: &PgPool) -> Result<(), sqlx::Error> {
    use citadel_runtime::runtime_metrics::RuntimeWork;
    for (family, query) in [
        (
            RuntimeWork::BuildQueue,
            "SELECT count(*)::bigint,coalesce(greatest(extract(epoch FROM (CURRENT_TIMESTAMP-min(queuedat)))*1000000,0),0)::bigint FROM buildruns WHERE status='Queued'",
        ),
        (
            RuntimeWork::BackupQueue,
            "SELECT count(*)::bigint,coalesce(greatest(extract(epoch FROM (CURRENT_TIMESTAMP-min(queuedat)))*1000000,0),0)::bigint FROM backupruns WHERE status='Queued'",
        ),
        (
            RuntimeWork::RestoreQueue,
            "SELECT count(*)::bigint,coalesce(greatest(extract(epoch FROM (CURRENT_TIMESTAMP-min(queuedat)))*1000000,0),0)::bigint FROM backuprestoreruns WHERE status='Queued'",
        ),
    ] {
        let (count, age): (i64, i64) = sqlx::query_as(query).fetch_one(pool).await?;
        family.queued(count as usize);
        family.age(std::time::Duration::from_micros(age as u64));
    }
    let (builds, data, unresolved): (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM buildexecutorreservations),(SELECT count(*) FROM dataexecutorreservations),(SELECT count(*) FROM buildexecutorreservations WHERE uncertain)+(SELECT count(*) FROM dataexecutorreservations d WHERE EXISTS(SELECT 1 FROM backupruns r WHERE r.id=d.ownerid AND r.status NOT IN ('Queued','Preparing','Running','ApplyingRetention')) OR EXISTS(SELECT 1 FROM backuprestoreruns r WHERE r.id=d.ownerid AND r.status NOT IN ('Queued','Preparing','Running')) OR EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.ownerrunid=d.ownerid AND l.expiresat<CURRENT_TIMESTAMP))")
        .fetch_one(pool).await?;
    RuntimeWork::BuildReservations.queued(builds as usize);
    RuntimeWork::DataReservations.queued(data as usize);
    RuntimeWork::ExecutionCleanupPending.queued(unresolved as usize);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::STATS_RETENTION_SQL;
    use sqlx::{Connection, Executor, PgConnection};

    #[tokio::test]
    #[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
    async fn stats_retention_uses_index_bounds_and_preserves_retention_policy() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let mut conn = PgConnection::connect(&url).await.unwrap();
        // Session-local tables: no application data, migrations, or persistent fixtures.
        for (table, sql) in [
            "workloadstats",
            "containerstats",
            "platformstats",
            "swarmservicestats",
        ]
        .into_iter()
        .zip(STATS_RETENTION_SQL)
        {
            let mut tx = conn.begin().await.unwrap();
            tx.execute(sqlx::AssertSqlSafe(format!(
                "CREATE TEMP TABLE {table} (id uuid PRIMARY KEY, created bigint NOT NULL, alertpending boolean NOT NULL DEFAULT false) ON COMMIT DROP;
                 CREATE INDEX ON {table}(created);
                 INSERT INTO {table}(id,created) SELECT gen_random_uuid(),floor(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint-n FROM generate_series(1,30000) n;
                 INSERT INTO {table}(id,created) SELECT gen_random_uuid(),floor(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint-700000-n FROM generate_series(1,10) n;
                 ANALYZE {table}"
            )))
            .await
            .unwrap();
            let plan: Vec<(String,)> =
                sqlx::query_as(sqlx::AssertSqlSafe(format!("EXPLAIN {sql}")))
                    .fetch_all(&mut *tx)
                    .await
                    .unwrap();
            assert!(
                plan.iter()
                    .any(|(line,)| line.contains("Index Cond: (created <")),
                "{table}: retention must bound the timestamp index, not filter every retained row: {plan:?}"
            );
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut *tx)
                    .await
                    .unwrap()
                    .rows_affected(),
                10
            );

            // Compare with the original fractional cutoff inside one transaction so
            // CURRENT_TIMESTAMP is fixed. Include timestamps on either side of it.
            tx.execute(sqlx::AssertSqlSafe(format!(
                "TRUNCATE {table};
                 INSERT INTO {table}(id,created) SELECT gen_random_uuid(),floor(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint+n FROM generate_series(-1,1) n"
            )))
            .await
            .unwrap();
            let expected: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {table} WHERE created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut *tx)
                    .await
                    .unwrap()
                    .rows_affected(),
                expected as u64,
                "{table}: the strict seven-day boundary must be unchanged"
            );

            tx.execute(sqlx::AssertSqlSafe(format!(
                "TRUNCATE {table};
                 INSERT INTO {table}(id,created) SELECT gen_random_uuid(),floor(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint-700000-n FROM generate_series(1,5001) n;
                 INSERT INTO {table}(id,created) VALUES(gen_random_uuid(),ceil(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint)"
            )))
            .await
            .unwrap();
            if table == "platformstats" {
                tx.execute("INSERT INTO platformstats(id,created,alertpending) VALUES(gen_random_uuid(),0,true)")
                    .await
                    .unwrap();
            }
            for expected in [5000, 1, 0] {
                assert_eq!(
                    sqlx::query(sql)
                        .execute(&mut *tx)
                        .await
                        .unwrap()
                        .rows_affected(),
                    expected,
                    "{table}: each pass must remain bounded and preserve fresh/pending samples"
                );
            }
            let remaining: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(remaining, if table == "platformstats" { 2 } else { 1 });
            tx.commit().await.unwrap();
        }
    }
}
