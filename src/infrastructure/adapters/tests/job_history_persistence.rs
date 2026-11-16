use citadel_adapters::{
    external::instance_jobs::RuntimeInstanceJobs,
    persistence::postgres::instance::PostgresJobHistory,
};
use citadel_instance::{
    JobDiagnosticSource, JobHistoryBatch, JobHistoryMaintenance, JobHistoryRepository,
    JobHistorySession, history_buckets,
};
use citadel_runtime::jobs::{JobCatalog, JobOutcome, JobOwnership};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn history_preserves_sessions_retries_atomically_and_bounds_retention() {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql("TRUNCATE jobdiagnosticevents,jobdiagnosticbuckets,jobdiagnosticsessions")
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresJobHistory(pool.clone());
    let catalog = JobCatalog::new(Uuid::now_v7());
    let job = catalog.register("stats", "Maintenance", JobOwnership::Coordinator);
    let target = citadel_runtime::jobs::JobTarget {
        resource_type: "Platform".into(),
        resource_id: Uuid::now_v7(),
        operation_id: None,
        container_id: Some(uuid::Uuid::now_v7()),
    };
    job.begin()
        .failure_context("Measurements could not be collected.", target.clone())
        .finish(JobOutcome::Failed, None);
    job.begin()
        .finish(JobOutcome::Productive, Some((1200, "samples")));
    let c = RuntimeInstanceJobs(catalog).capture(0);
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let mut batch = JobHistoryBatch {
        sequence: 1,
        buckets: history_buckets(None, &c.snapshot),
        events: c.events,
        session: JobHistorySession {
            core_id: Uuid::now_v7(),
            version: "test".into(),
            heartbeat_at: now,
            stopped_at: None,
            lost_events: 0,
            snapshot: c.snapshot,
        },
        checkpoint: true,
    };
    store.persist(&batch).await.unwrap();
    store.persist(&batch).await.unwrap();
    let history = store.read(Some("stats"), 24).await.unwrap();
    assert_eq!(history.events.len(), 2);
    assert_eq!(history.buckets.len(), 1);
    assert_eq!(history.buckets[0].units, Some(1200));
    assert_eq!(history.sessions.len(), 1);
    assert_eq!(history.events[0].event.kind, "recovered");
    assert!(history.events[0].event.problem.is_none());
    assert_eq!(
        history.events[1]
            .event
            .problem
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .resource_id,
        target.resource_id
    );
    assert_eq!(
        history.sessions[0].snapshot.jobs[0].last_completed_units,
        Some(1200)
    );
    assert_eq!(
        history.sessions[0].snapshot.jobs[0].last_problem,
        history.events[1].event.problem
    );
    assert!(
        store
            .read(Some("another-job"), 24)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    assert!(store.read(None, 24).await.unwrap().buckets.is_empty());
    // A newer batch is accepted even when the clock has not advanced.
    batch.sequence += 1;
    batch.session.snapshot.revision += 1;
    store.persist(&batch).await.unwrap();
    assert_eq!(
        store.read(Some("stats"), 24).await.unwrap().sessions[0]
            .snapshot
            .revision,
        batch.session.snapshot.revision
    );
    // A heartbeat need not rewrite the large snapshot or its work buckets.
    batch.session.heartbeat_at += 1;
    batch.sequence += 1;
    batch.session.snapshot.revision += 100;
    batch.checkpoint = false;
    batch.buckets.clear();
    store.persist(&batch).await.unwrap();
    let retained = store.read(Some("stats"), 24).await.unwrap();
    assert_ne!(
        retained.sessions[0].snapshot.revision,
        batch.session.snapshot.revision
    );
    assert_eq!(retained.buckets[0].units, Some(1200));
    // A failure after updating the heartbeat rolls the entire batch back.
    sqlx::raw_sql("CREATE FUNCTION reject_job_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture failure'; END $$; CREATE TRIGGER reject_job_event BEFORE INSERT ON jobdiagnosticevents FOR EACH ROW EXECUTE FUNCTION reject_job_event();").execute(&pool).await.unwrap();
    batch.session.heartbeat_at += 1;
    batch.sequence += 1;
    assert!(store.persist(&batch).await.is_err());
    assert_eq!(
        store.read(None, 24).await.unwrap().sessions[0].heartbeat_at,
        now + 1
    );
    sqlx::raw_sql(
        "DROP TRIGGER reject_job_event ON jobdiagnosticevents; DROP FUNCTION reject_job_event();",
    )
    .execute(&pool)
    .await
    .unwrap();
    store.persist(&batch).await.unwrap();
    // A different startup never overwrites the old one, even on the same Core.
    let previous_boot = batch.session.snapshot.boot_id;
    batch.session.snapshot.boot_id = Uuid::now_v7();
    batch.session.heartbeat_at += 1;
    batch.sequence += 1;
    batch.events.clear();
    store.persist(&batch).await.unwrap();
    let sessions = store.read(None, 24).await.unwrap().sessions;
    assert_eq!(sessions.len(), 2);
    assert!(sessions.iter().any(|s| s.snapshot.boot_id == previous_boot));
    // Old diagnostic rows expire without touching current sessions.
    let old = (now as i64 - 8 * 86_400_000) / 300_000 * 300_000;
    sqlx::query("UPDATE jobdiagnosticevents SET observed=$1 WHERE bootid=$2")
        .bind(old)
        .bind(previous_boot)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE jobdiagnosticbuckets SET bucket=$1 WHERE bootid=$2")
        .bind(old)
        .bind(previous_boot)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE jobdiagnosticsessions SET heartbeat=$1 WHERE bootid=$2")
        .bind(old)
        .bind(previous_boot)
        .execute(&pool)
        .await
        .unwrap();
    batch.session.heartbeat_at += 1;
    batch.sequence += 1;
    store.persist(&batch).await.unwrap();
    store.maintain().await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobdiagnosticsessions WHERE bootid=$1")
            .bind(previous_boot)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(store.read(None, 24).await.unwrap().sessions.len(), 1);
    pool.close().await;
}

#[test]
fn diagnostic_identity_survives_restarts_without_replacing_corrupt_identity() {
    use citadel_adapters::external::instance_jobs::diagnostic_core_id;
    let directory = tempfile::tempdir().unwrap();
    let id = diagnostic_core_id(directory.path()).unwrap();
    assert_eq!(diagnostic_core_id(directory.path()).unwrap(), id);
    let path = directory.path().join("core-diagnostics-id");
    std::fs::write(&path, "invalid identity").unwrap();
    assert!(diagnostic_core_id(directory.path()).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "invalid identity");
}

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn compaction_preserves_totals_retries_gaps_and_protects_mutable_hours() {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql("TRUNCATE jobdiagnosticevents,jobdiagnosticbuckets,jobdiagnosticsessions")
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresJobHistory(pool.clone());
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let hour = now / 3_600_000 * 3_600_000 - 48 * 3_600_000;
    let catalog = JobCatalog::new(Uuid::now_v7());
    let job = catalog.register("audit", "Maintenance", JobOwnership::Coordinator);
    job.begin()
        .finish(JobOutcome::Productive, Some((10, "rows")));
    let capture = RuntimeInstanceJobs(catalog).capture(0);
    let template = history_buckets(None, &capture.snapshot).remove(0);
    let mut batch = JobHistoryBatch {
        sequence: 1,
        session: JobHistorySession {
            core_id: Uuid::now_v7(),
            version: "test".into(),
            snapshot: capture.snapshot,
            heartbeat_at: now,
            stopped_at: None,
            lost_events: 0,
        },
        buckets: (0..12)
            .map(|i| citadel_instance::JobHistoryBucket {
                at: hour + i * 300_000,
                through: hour + i * 300_000 + 30_000,
                iterations: i + 2,
                productive: i + 1,
                no_work: 1,
                failures: u64::from(i == 3),
                productive_ms: (i + 1) * 10,
                no_work_ms: 2,
                elapsed_ms: (i + 1) * 10 + 2,
                units: Some(i + 1),
                backlog: Some(12 - i),
                backlog_is_lower_bound: i == 11,
                gap: i == 5,
                ..template.clone()
            })
            .chain(std::iter::once(template.clone()))
            .collect(),
        events: vec![],
        checkpoint: true,
    };
    // Keep the last saved checkpoint hour mutable, even if it is two days old.
    batch.session.snapshot.observed_at = hour + 3_500_000;
    store.persist(&batch).await.unwrap();
    assert_eq!(store.maintain().await.unwrap(), 0);
    batch.sequence += 1;
    batch.session.snapshot.observed_at = now;
    store.persist(&batch).await.unwrap();
    // Fault after source deletion must roll back the entire compaction.
    sqlx::raw_sql("CREATE FUNCTION reject_hour() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.resolutionms=3600000 THEN RAISE EXCEPTION 'fixture rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_hour BEFORE INSERT ON jobdiagnosticbuckets FOR EACH ROW EXECUTE FUNCTION reject_hour();").execute(&pool).await.unwrap();
    assert!(store.maintain().await.is_err());
    assert_eq!(
        store.read(Some("audit"), 168).await.unwrap().buckets.len(),
        13
    );
    // Cleanup failure does not prevent an ordinary diagnostic heartbeat.
    batch.sequence += 1;
    batch.checkpoint = false;
    let saved = std::mem::take(&mut batch.buckets);
    store.persist(&batch).await.unwrap();
    sqlx::raw_sql("DROP TRIGGER reject_hour ON jobdiagnosticbuckets; DROP FUNCTION reject_hour();")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(store.maintain().await.unwrap(), 12);
    assert_eq!(store.maintain().await.unwrap(), 0);
    let history = store.read(Some("audit"), 168).await.unwrap();
    assert_eq!(history.buckets.len(), 2);
    let hourly = history.buckets.iter().find(|b| b.at == hour).unwrap();
    assert_eq!(hourly.resolution_ms, 3_600_000);
    assert_eq!(
        (
            hourly.productive,
            hourly.productive_ms,
            hourly.no_work,
            hourly.no_work_ms
        ),
        (78, 780, 12, 24)
    );
    assert_eq!(
        (hourly.iterations, hourly.elapsed_ms, hourly.failures),
        (90, 804, 1)
    );
    assert_eq!(
        (hourly.units, hourly.backlog, hourly.backlog_is_lower_bound),
        (Some(78), Some(1), true)
    );
    assert!(hourly.gap);
    assert_eq!(
        store.read(Some("audit"), 24).await.unwrap().buckets.len(),
        1
    );
    // An ambiguous old flush cannot recreate fine rows. A genuinely new late
    // mutation is rejected atomically rather than corrupting the sealed hour.
    batch.buckets = saved;
    batch.checkpoint = true;
    store.persist(&batch).await.unwrap();
    batch.sequence += 1;
    assert!(store.persist(&batch).await.is_err());
    assert_eq!(
        store.read(Some("audit"), 168).await.unwrap().buckets.len(),
        2
    );
    // A new Core startup keeps a separate history, even for the same job/hour.
    batch.session.snapshot.boot_id = Uuid::now_v7();
    for b in &mut batch.buckets {
        b.boot_id = batch.session.snapshot.boot_id;
    }
    batch.buckets[1].units = None;
    batch.buckets[2].unit = Some("different units".into());
    batch.buckets.remove(5);
    store.persist(&batch).await.unwrap();
    assert_eq!(store.maintain().await.unwrap(), 11);
    let history = store.read(Some("audit"), 168).await.unwrap();
    assert_eq!(history.buckets.len(), 4);
    let hourly = history
        .buckets
        .iter()
        .find(|b| b.boot_id == batch.session.snapshot.boot_id && b.at == hour)
        .unwrap();
    assert_eq!(hourly.units, None);
    assert_eq!(hourly.unit, None);
    assert!(hourly.gap);
    // A clean shutdown seals even the final checkpoint hour.
    batch.session.snapshot.boot_id = Uuid::now_v7();
    batch.session.snapshot.observed_at = hour + 300_000;
    batch.session.stopped_at = Some(hour + 300_000);
    batch.buckets = vec![citadel_instance::JobHistoryBucket {
        boot_id: batch.session.snapshot.boot_id,
        at: hour,
        through: hour + 30_000,
        ..template
    }];
    store.persist(&batch).await.unwrap();
    assert_eq!(store.maintain().await.unwrap(), 1);
    let history = store.read(Some("audit"), 168).await.unwrap();
    assert_eq!(
        history
            .buckets
            .iter()
            .find(|b| b.boot_id == batch.session.snapshot.boot_id)
            .unwrap()
            .resolution_ms,
        3_600_000
    );
}

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_STATISTICS_DATABASE_URL; seven-day 50-job workload"]
async fn seven_day_job_history_compaction_audit() {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql("TRUNCATE jobdiagnosticevents,jobdiagnosticbuckets,jobdiagnosticsessions; CREATE EXTENSION IF NOT EXISTS pg_stat_statements; SELECT pg_stat_statements_reset();").execute(&pool).await.unwrap();
    let store = PostgresJobHistory(pool.clone());
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let end = now / 3_600_000 * 3_600_000;
    let catalog = JobCatalog::new(Uuid::now_v7());
    let job = catalog.register("audit-1", "Maintenance", JobOwnership::Coordinator);
    job.begin()
        .finish(JobOutcome::Productive, Some((10, "rows")));
    let capture = RuntimeInstanceJobs(catalog).capture(0);
    let template = history_buckets(None, &capture.snapshot).remove(0);
    let boot = capture.snapshot.boot_id;
    let batch = JobHistoryBatch {
        sequence: 1,
        session: JobHistorySession {
            core_id: Uuid::now_v7(),
            version: "audit".into(),
            snapshot: capture.snapshot,
            heartbeat_at: now,
            stopped_at: None,
            lost_events: 0,
        },
        buckets: vec![],
        events: vec![],
        checkpoint: true,
    };
    store.persist(&batch).await.unwrap();
    sqlx::query("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,data) SELECT $1,'audit-'||j,t,$2::jsonb||jsonb_build_object('jobId','audit-'||j,'at',t,'through',t+30000) FROM generate_series(1,50) j CROSS JOIN generate_series($3::bigint-604800000,$3::bigint-300000,300000) t")
        .bind(boot).bind(sqlx::types::Json(template)).bind(end as i64).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE jobdiagnosticbuckets; ANALYZE jobdiagnosticsessions;")
        .execute(&pool)
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM jobdiagnosticbuckets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, 100_800);
    // Attribute SQL WAL to this disposable database. Cluster LSN differences
    // also include unrelated databases (for example concurrent migration tests).
    let wal: f64 = sqlx::query_scalar("SELECT coalesce(sum(wal_bytes),0)::float8 FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())")
        .fetch_one(&pool).await.unwrap();
    let started = std::time::Instant::now();
    let mut passes = 0;
    while store.maintain().await.unwrap() != 0 {
        passes += 1;
        assert!(passes < 10_000, "maintenance must converge");
    }
    let elapsed = started.elapsed();
    let wal_bytes: f64 = sqlx::query_scalar("SELECT coalesce(sum(wal_bytes),0)::float8 FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())")
        .fetch_one(&pool).await.unwrap();
    let wal_bytes = wal_bytes - wal;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM jobdiagnosticbuckets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        (21_500..=22_200).contains(&after),
        "unexpected retained window count: {after}"
    );
    let started = std::time::Instant::now();
    let history = store.read(Some("audit-1"), 168).await.unwrap();
    let read_elapsed = started.elapsed();
    assert!(!history.truncated);
    assert!(history.buckets.len() <= 444);
    let live_bytes: i64 =
        sqlx::query_scalar("SELECT sum(pg_column_size(data))::bigint FROM jobdiagnosticbuckets")
            .fetch_one(&pool)
            .await
            .unwrap();
    let allocated: i64 =
        sqlx::query_scalar("SELECT pg_total_relation_size('jobdiagnosticbuckets')")
            .fetch_one(&pool)
            .await
            .unwrap();
    println!(
        "JOBS_AUDIT before={before} after={after} passes={passes} compaction_ms={} sql_wal_bytes={wal_bytes} live_json_bytes={live_bytes} allocated_bytes={allocated} seven_day_windows={} read_ms={}",
        elapsed.as_millis(),
        history.buckets.len(),
        read_elapsed.as_micros() as f64 / 1000.0
    );
}
