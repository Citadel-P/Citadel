use super::*;
use citadel_adapters::{
    external::backups::restic::DockerResticBackupExecutor,
    persistence::postgres::backups::{
        PostgresBackupPersistence, source_planner::PostgresBackupSourcePlanner,
    },
};
use citadel_backups::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

struct NoSecrets;
impl BackupSecretResolver for NoSecrets {
    fn resolve(
        &self,
        _: Uuid,
        _: Uuid,
    ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>> {
        panic!("Rejected fixture work must never execute")
    }
}

struct BlockingAuthorization {
    entered: AtomicUsize,
    shutdown: CancellationToken,
}
impl BackupRunAuthorizer for BlockingAuthorization {
    fn authorize_backup<'a>(
        &'a self,
        _: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async {
            self.entered.fetch_add(1, Ordering::SeqCst);
            self.shutdown.cancelled().await;
            Err(BackupError::Conflict(
                "Fixture authorization rejected".into(),
            ))
        })
    }
    fn authorize_restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        unreachable!()
    }
    fn authorize_plan<'a>(
        &'a self,
        _: &'a BackupClaim,
        _: &'a BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        unreachable!()
    }
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn dispatcher_bounds_claims_and_drains_controllers_on_shutdown() {
    let database = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&database)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let secret = Uuid::now_v7();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("worker-secret-{secret}"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global','00000000-0000-0000-0000-000000000000')").bind(secret).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).bind(format!("worker-platform-{platform}")).execute(&pool).await.unwrap();
    let store = Arc::new(PostgresBackupPersistence::new(pool.clone()));
    let mut runs = Vec::new();
    for _ in 0..3 {
        let name = format!("worker-{}", Uuid::now_v7());
        let repository = store.create_repository(actor, &BackupRepositoryConfiguration {
            name: name.clone(),
            spec: serde_json::from_value(serde_json::json!({"$type":"FileSystem","location":"Core","path":"/tmp/never-executed"})).unwrap(),
            password_secret_id: secret,
        }).await.unwrap();
        let policy = store.create_policy(actor, &BackupPolicyConfiguration {
            name: name.clone(), description: None,
            source: serde_json::from_value(serde_json::json!({"$type":"DockerVolume","platformId":platform,"volumeName":name})).unwrap(),
            backup_repository_id: repository.id, enabled: true, cron: None, time_zone: None,
            webhook: None, keep_last_successful: Some(1), timeout_seconds: Some(60),
            alert_on_failure: false, run_as_actor_id: None, tag_ids: vec![],
        }).await.unwrap();
        runs.push(
            store
                .enqueue_backup(actor, policy.id, "Manual", None)
                .await
                .unwrap()
                .id,
        );
    }
    let shutdown = CancellationToken::new();
    let authorization = Arc::new(BlockingAuthorization {
        entered: AtomicUsize::new(0),
        shutdown: shutdown.clone(),
    });
    let service = Arc::new(
        BackupService::new(
            store.clone(),
            Arc::new(DockerResticBackupExecutor::new(
                "must-not-run",
                "unused",
                Arc::new(NoSecrets),
                4096,
                pool.clone(),
            )),
            Arc::new(PostgresBackupSourcePlanner::new(pool.clone())),
            chrono::Duration::minutes(5),
            authorization.clone(),
        )
        .with_tracking_slots(2),
    );
    let worker = tokio::spawn(backup_runs(
        shutdown.clone(),
        service,
        crate::config::BackupWorkerConfig {
            enabled: true,
            tracking_slots: 2,
            poll_interval: Duration::from_secs(30),
            schedule_interval: Duration::from_secs(30),
        },
        tokio::sync::watch::channel(()).1,
    ));
    let started = tokio::time::timeout(Duration::from_secs(5), async {
        while authorization.entered.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    // Always stop the worker before assertions so failed fixtures do not detach tasks.
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    started.expect("both controllers must start without waiting for a polling interval");
    assert_eq!(authorization.entered.load(Ordering::SeqCst), 2);
    let statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM backupruns WHERE id=ANY($1) ORDER BY status")
            .bind(&runs)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(statuses, ["Failed", "Failed", "Queued"]);
    let leases: i64 =
        sqlx::query_scalar("SELECT count(*) FROM backuprepositoryleases WHERE ownerrunid=ANY($1)")
            .bind(&runs)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        leases, 0,
        "shutdown must finish controllers and their persistence"
    );
    pool.close().await;
}
