//! Maintenance runs independently of dispatch being enabled and of new statistics writes.
use sqlx::PgPool;

// Keep the indexed bigint column uncast. EXTRACT returns numeric, which otherwise
// makes PostgreSQL filter the retained history instead of using an index bound.
// CEIL preserves `integer timestamp < fractional cutoff`, including whole seconds.
// Reuse the locked tuple locations within this statement. Matching a batch by ID
// can make the DELETE scan the entire retained table via a hash semi join.
// An array InitPlan feeds the TID scan directly: even a ctid join can otherwise
// become a hash join over every retained row when the estimated batch is large.
// These are ordinary, non-partitioned tables; never persist ctids across statements.
// See https://www.postgresql.org/docs/18/sql-delete.html for bounded DELETEs.
const STATS_RETENTION_SQL: [&str; 5] = [
    "WITH expired AS MATERIALIZED (
        SELECT ctid FROM platformnodeiostats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint
        ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED
    ) DELETE FROM platformnodeiostats target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))",
    "WITH expired AS MATERIALIZED (
        SELECT ctid FROM workloadstats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint
        ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED
    ) DELETE FROM workloadstats target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))",
    "WITH expired AS MATERIALIZED (
        SELECT ctid FROM containerstats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint
        ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED
    ) DELETE FROM containerstats target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))",
    "WITH expired AS MATERIALIZED (
        SELECT ctid FROM platformstats WHERE NOT alertpending AND created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint
        ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED
    ) DELETE FROM platformstats target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))",
    "WITH expired AS MATERIALIZED (
        SELECT ctid FROM swarmservicestats WHERE created<CEIL(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800)::bigint
        ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED
    ) DELETE FROM swarmservicestats target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))",
];

const HEALTH_RETENTION_SQL: &str = "WITH expired AS MATERIALIZED (SELECT ctid FROM containerhealthtransitions WHERE recordedat < now() - interval '7 days' ORDER BY recordedat LIMIT 1000 FOR UPDATE SKIP LOCKED) DELETE FROM containerhealthtransitions target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))";
const ACTIVITY_RETENTION_SQL: &str = "WITH expired AS MATERIALIZED (SELECT ctid FROM activityevents WHERE createdat<CURRENT_TIMESTAMP-INTERVAL '90 days' ORDER BY createdat LIMIT 5000 FOR UPDATE SKIP LOCKED) DELETE FROM activityevents target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))";
const SCRIPT_RETENTION_SQL: &str = "WITH expired AS MATERIALIZED (SELECT r.ctid FROM scriptruns r WHERE finishedat<CURRENT_TIMESTAMP-INTERVAL '90 days' AND status NOT IN('Queued','Running') AND NOT EXISTS(SELECT 1 FROM scripts a WHERE a.currentrunid=r.id) ORDER BY finishedat LIMIT 5000 FOR UPDATE OF r SKIP LOCKED) DELETE FROM scriptruns target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))";
const TOKEN_EXPIRY_SQL: &str = "WITH expired AS MATERIALIZED (SELECT ctid FROM refreshtokens WHERE expiresat<CURRENT_TIMESTAMP ORDER BY expiresat LIMIT 5000 FOR UPDATE SKIP LOCKED) DELETE FROM refreshtokens target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))";
const BUILD_RETENTION_SQL: &str = "WITH expired AS MATERIALIZED (SELECT r.ctid FROM buildruns r WHERE r.completedat<CURRENT_TIMESTAMP-make_interval(days=>$1) AND r.status IN('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND NOT EXISTS(SELECT 1 FROM buildexecutorreservations reservation WHERE reservation.runid=r.id) AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=r.id) AND NOT EXISTS(SELECT 1 FROM buildprojects p WHERE p.currentrunid=r.id) AND NOT EXISTS(SELECT 1 FROM deployments d WHERE jsonb_path_exists(d.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) AND NOT EXISTS(SELECT 1 FROM stackreleases s WHERE jsonb_path_exists(s.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',r.id::text))) ORDER BY r.completedat LIMIT 1000 FOR UPDATE OF r SKIP LOCKED) DELETE FROM buildruns target WHERE target.ctid=ANY(ARRAY(SELECT ctid FROM expired))";

pub async fn reconcile(pool: &PgPool) -> Result<Vec<&'static str>, sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::Recovery.start();
    let mut changed = Vec::new();
    let mut tx = pool.begin().await?;
    // Existing recovery predicates protect live runs until their execution deadlines.
    let before = chrono::Utc::now();
    if !crate::persistence::postgres::scripts::recover_interrupted(&mut tx, before)
        .await
        .map_err(protocol)?
        .is_empty()
    {
        changed.extend(["Script"]);
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
            "Script",
            "UPDATE scripts a SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE controlstate='Processing' AND NOT EXISTS(SELECT 1 FROM scriptexecutorreservations held WHERE held.runid=a.currentrunid) AND NOT EXISTS(SELECT 1 FROM scriptruns r WHERE r.id=a.currentrunid AND r.status IN('Queued','Running'))",
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
    let mut deleted = sqlx::query(HEALTH_RETENTION_SQL)
        .execute(pool)
        .await?
        .rows_affected();
    citadel_runtime::runtime_metrics::RuntimeWork::ContainerHealthRetention.units(deleted);
    // Bound each transaction. Repeated scheduled batches drain backlogs while keeping
    // locks short and retaining every active run and referenced build artifact.
    for sql in STATS_RETENTION_SQL
        .into_iter()
        .chain([ACTIVITY_RETENTION_SQL, SCRIPT_RETENTION_SQL])
    {
        deleted += sqlx::query(sql).execute(pool).await?.rows_affected();
    }
    if let Some(days) = build_retention_days.filter(|days| *days > 0) {
        deleted += sqlx::query(BUILD_RETENTION_SQL)
            .bind(days)
            .execute(pool)
            .await?
            .rows_affected();
    }
    citadel_runtime::runtime_metrics::RuntimeWork::Retention.units(deleted);
    Ok(deleted)
}
/// Refresh tokens have a separate expiry cadence. Backup leases are ownership
/// evidence: only verified cleanup may release them, regardless of expiry.
pub async fn expire_leases(pool: &PgPool) -> Result<(), sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::LeaseExpiry.start();
    let count = sqlx::query(TOKEN_EXPIRY_SQL)
        .execute(pool)
        .await?
        .rows_affected();
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
    crate::persistence::postgres::scripts::recover_interrupted_with_mode(&mut tx, before, true)
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
    async fn other_retention_preserves_protected_runs_and_bounds_deletes() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let mut conn = PgConnection::connect(&url).await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        tx.execute(
            "CREATE TEMP TABLE scripts(currentrunid uuid) ON COMMIT DROP;
            CREATE TEMP TABLE buildprojects(currentrunid uuid) ON COMMIT DROP;
            CREATE TEMP TABLE buildexecutorreservations(runid uuid) ON COMMIT DROP;
            CREATE TEMP TABLE buildcompletionqueue(buildrunid uuid) ON COMMIT DROP;
            CREATE TEMP TABLE deployments(spec jsonb) ON COMMIT DROP;
            CREATE TEMP TABLE stackreleases(spec jsonb) ON COMMIT DROP;",
        )
        .await
        .unwrap();
        for (table, time, query, limit) in [
            (
                "containerhealthtransitions",
                "recordedat",
                super::HEALTH_RETENTION_SQL,
                1000,
            ),
            (
                "activityevents",
                "createdat",
                super::ACTIVITY_RETENTION_SQL,
                5000,
            ),
            (
                "scriptruns",
                "finishedat",
                super::SCRIPT_RETENTION_SQL,
                5000,
            ),
            ("buildruns", "completedat", super::BUILD_RETENTION_SQL, 1000),
            ("refreshtokens", "expiresat", super::TOKEN_EXPIRY_SQL, 5000),
        ] {
            tx.execute(sqlx::AssertSqlSafe(format!(
                "CREATE TEMP TABLE {table}(id uuid PRIMARY KEY, {time} timestamptz NOT NULL, status text NOT NULL DEFAULT 'Succeeded') ON COMMIT DROP;
                 CREATE INDEX ON {table}({time});
                 INSERT INTO {table}(id,{time}) SELECT gen_random_uuid(),now()+interval '1 day' FROM generate_series(1,100000);
                 INSERT INTO {table}(id,{time}) SELECT gen_random_uuid(),now()-interval '100 days' FROM generate_series(1,{limit}+1);
                 ANALYZE {table}"
            ))).await.unwrap();
            let mut protected = 0;
            if table == "scriptruns" || table == "buildruns" {
                // Neither active states nor the current run may be pruned, even
                // if a stale completion timestamp is present.
                let states = if table == "scriptruns" {
                    vec!["Queued", "Running"]
                } else {
                    vec!["Queued", "Preparing", "Running"]
                };
                for state in states {
                    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {table}(id,{time},status) VALUES(gen_random_uuid(),now()-interval '101 days',$1)")))
                        .bind(state).execute(&mut *tx).await.unwrap();
                    protected += 1;
                }
                let owner = if table == "scriptruns" {
                    "scripts"
                } else {
                    "buildprojects"
                };
                tx.execute(sqlx::AssertSqlSafe(format!("WITH run AS (INSERT INTO {table}(id,{time}) VALUES(gen_random_uuid(),now()-interval '101 days') RETURNING id) INSERT INTO {owner}(currentrunid) SELECT id FROM run"))).await.unwrap();
                protected += 1;
            }
            if table == "buildruns" {
                for (owner, column) in [
                    ("buildexecutorreservations", "runid"),
                    ("buildcompletionqueue", "buildrunid"),
                    ("deployments", "spec"),
                    ("stackreleases", "spec"),
                ] {
                    let value = if column == "spec" {
                        "jsonb_build_object('nested', jsonb_build_array(jsonb_build_object('buildRunId', id::text)))"
                    } else {
                        "id"
                    };
                    tx.execute(sqlx::AssertSqlSafe(format!("WITH run AS (INSERT INTO buildruns(id,completedat) VALUES(gen_random_uuid(),now()-interval '101 days') RETURNING id) INSERT INTO {owner}({column}) SELECT {value} FROM run"))).await.unwrap();
                    protected += 1;
                }
            }
            let explain = format!("EXPLAIN (ANALYZE, FORMAT JSON) {query}");
            let statement =
                sqlx::query_scalar::<_, serde_json::Value>(sqlx::AssertSqlSafe(explain.as_str()));
            let statement = if table == "buildruns" {
                statement.bind(90_i32)
            } else {
                statement
            };
            let plan = statement.fetch_one(&mut *tx).await.unwrap();
            fn bounded_scans(plan: &serde_json::Value, table: &str, limit: i64) {
                if plan["Relation Name"] == table {
                    let visited = (plan["Actual Rows"].as_f64().unwrap_or(0.)
                        + plan["Rows Removed by Filter"].as_f64().unwrap_or(0.))
                        * plan["Actual Loops"].as_f64().unwrap_or(1.);
                    assert!(
                        visited <= (limit + 10) as f64,
                        "{table}: unexpectedly scanned retained history: {plan}"
                    );
                }
                if let Some(children) = plan["Plans"].as_array() {
                    for child in children {
                        bounded_scans(child, table, limit);
                    }
                }
            }
            bounded_scans(&plan[0]["Plan"], table, limit);
            let remaining: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(
                remaining,
                100001 + protected,
                "{table}: first batch must be bounded"
            );
            for expected in [1, 0] {
                let statement = sqlx::query(query);
                let statement = if table == "buildruns" {
                    statement.bind(90_i32)
                } else {
                    statement
                };
                assert_eq!(
                    statement.execute(&mut *tx).await.unwrap().rows_affected(),
                    expected
                );
            }
        }
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
    async fn stats_retention_uses_index_bounds_and_preserves_retention_policy() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let mut conn = PgConnection::connect(&url).await.unwrap();
        // Session-local tables: no application data, migrations, or persistent fixtures.
        for (table, sql) in [
            "platformnodeiostats",
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

    #[tokio::test]
    #[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
    async fn stats_retention_large_history_does_not_scan_retained_rows() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let mut conn = PgConnection::connect(&url).await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        tx.execute(
            "CREATE TEMP TABLE workloadstats(id uuid PRIMARY KEY, created bigint NOT NULL, payload text NOT NULL) ON COMMIT DROP;
             CREATE INDEX ON workloadstats(created);
             INSERT INTO workloadstats SELECT gen_random_uuid(),ceil(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint-n,repeat('x',192) FROM generate_series(1,100000) n;
             INSERT INTO workloadstats SELECT gen_random_uuid(),floor(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint-700000-n,repeat('x',192) FROM generate_series(1,1080) n;
             ANALYZE workloadstats"
        ).await.unwrap();
        // Executed only on session-local fixtures. A bounded output alone does not
        // prove bounded work: the old ID semi join scanned all retained rows.
        let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "EXPLAIN (ANALYZE, FORMAT JSON) {}",
            STATS_RETENTION_SQL[1]
        )))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        fn check_scans(node: &serde_json::Value) {
            if node["Relation Name"] == "workloadstats" {
                let examined = (node["Actual Rows"].as_f64().unwrap_or(0.)
                    + node["Rows Removed by Filter"].as_f64().unwrap_or(0.))
                    * node["Actual Loops"].as_f64().unwrap_or(0.);
                assert!(
                    examined <= 5000.,
                    "small retention batch scanned retained history: {node}"
                );
            }
            if let Some(children) = node["Plans"].as_array() {
                for child in children {
                    check_scans(child);
                }
            }
        }
        check_scans(&plan[0]["Plan"]);
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM workloadstats")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(remaining, 100000);
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
    async fn stats_retention_skips_locked_rows_and_reselects_updated_tuples() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let mut cleaner = PgConnection::connect(&url).await.unwrap();
        let mut locker = PgConnection::connect(&url).await.unwrap();
        // Two sessions need shared tables; isolate them in a disposable namespace.
        let schema = format!("retention_test_{}", uuid::Uuid::now_v7().simple());
        cleaner
            .execute(sqlx::AssertSqlSafe(format!(
                "CREATE SCHEMA {schema}; SET search_path TO {schema}"
            )))
            .await
            .unwrap();
        locker
            .execute(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
            .await
            .unwrap();
        // If SKIP LOCKED regresses, fail rather than hanging behind our own lock.
        cleaner
            .execute("SET lock_timeout='1s'; SET statement_timeout='5s'")
            .await
            .unwrap();
        for (table, sql) in [
            "platformnodeiostats",
            "workloadstats",
            "containerstats",
            "platformstats",
            "swarmservicestats",
        ]
        .into_iter()
        .zip(STATS_RETENTION_SQL)
        {
            cleaner.execute(sqlx::AssertSqlSafe(format!(
                "CREATE TABLE {table}(id integer PRIMARY KEY, created bigint NOT NULL, alertpending boolean NOT NULL DEFAULT false);
                 INSERT INTO {table}(id,created) VALUES (1,0),(2,0),(3,ceil(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))::bigint)"
            ))).await.unwrap();
            let old_location: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT ctid::text FROM {table} WHERE id=1"
            )))
            .fetch_one(&mut cleaner)
            .await
            .unwrap();
            let mut held = locker.begin().await.unwrap();
            let new_location: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "UPDATE {table} SET created=1 WHERE id=1 RETURNING ctid::text"
            )))
            .fetch_one(&mut *held)
            .await
            .unwrap();
            assert_ne!(old_location, new_location);
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut cleaner)
                    .await
                    .unwrap()
                    .rows_affected(),
                1,
                "{table}: delete the unlocked expired row without waiting"
            );
            let remaining: Vec<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT id FROM {table} ORDER BY id"
            )))
            .fetch_all(&mut cleaner)
            .await
            .unwrap();
            assert_eq!(remaining, vec![1, 3]);
            held.commit().await.unwrap();
            // A rolled-back pass must not lose its target or leave a remembered ctid.
            let mut rolled_back = cleaner.begin().await.unwrap();
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut *rolled_back)
                    .await
                    .unwrap()
                    .rows_affected(),
                1
            );
            rolled_back.rollback().await.unwrap();
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut cleaner)
                    .await
                    .unwrap()
                    .rows_affected(),
                1
            );
            assert_eq!(
                sqlx::query(sql)
                    .execute(&mut cleaner)
                    .await
                    .unwrap()
                    .rows_affected(),
                0
            );
            let remaining: Vec<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT id FROM {table} ORDER BY id"
            )))
            .fetch_all(&mut cleaner)
            .await
            .unwrap();
            assert_eq!(remaining, vec![3]);
        }
        cleaner
            .execute(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .await
            .unwrap();
    }
}
