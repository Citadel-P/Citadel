#![cfg(unix)]

use chrono::Utc;
use citadel_adapters::{
    backup_executor::DockerResticBackupExecutor, backup_store::PostgresBackupStore,
};
use citadel_backups::*;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_execution::{ProcessLimits, ProcessRequest, run};
use citadel_identity::SYSTEM_ACTOR_ID;
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::json;
use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

const IMAGE: &str = "restic/restic:0.18.1";
const PAYLOAD: &[u8] = b"Citadel restore must preserve the volume root.\n";
const ACCESS_KEY_ID: Uuid = Uuid::from_u128(710001);
const SECRET_KEY_ID: Uuid = Uuid::from_u128(710002);
struct Password;
impl BackupSecretResolver for Password {
    fn resolve(&self, id: Uuid) -> BoxFuture<'_, Result<Zeroizing<String>, BackupError>> {
        Box::pin(async move {
            Ok(Zeroizing::new(
                match id {
                    ACCESS_KEY_ID => "citadel-acceptance-access",
                    SECRET_KEY_ID => "citadel-acceptance-secret-not-for-production",
                    _ => "disposable-acceptance-password",
                }
                .into(),
            ))
        })
    }
}

async fn docker(args: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
    let mut request = ProcessRequest::new("docker")
        .args(args.iter().copied())
        .limits(ProcessLimits {
            timeout: Duration::from_secs(120),
            ..Default::default()
        });
    if let Some(stdin) = stdin {
        request = request.stdin(stdin.to_vec());
    }
    let output = run(request, &CancellationToken::new()).await.unwrap();
    assert!(
        output.succeeded(),
        "Docker fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

// Ports the volume round trip in BackupCompatibilityTests/SwarmBackupCompatibilityTests.
// This is real Docker + Restic + PostgreSQL, not the multi-node/Agent acceptance gate.
#[tokio::test]
#[ignore = "requires dedicated CITADEL_PHASE7_LOCAL_BACKUP_DATABASE_URL, Docker and restic/restic:0.18.1"]
async fn local_volume_backup_restores_root_data_and_persists_real_results() {
    volume_round_trip(None).await;
}

// Ports the S3/RustFS storage portion of WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode.
// Exact-node Agent routing has separate transport tests; this fixture uses Local Docker.
#[tokio::test]
#[ignore = "requires the dedicated RustFS/PostgreSQL fixture from Test-Phase7LocalBackup.ps1 -UseRustFs"]
async fn rustfs_volume_backup_restores_root_data_and_persists_real_results() {
    volume_round_trip(Some(
        std::env::var("CITADEL_PHASE7_RUSTFS_ENDPOINT").expect("RustFS fixture endpoint required"),
    ))
    .await;
}

async fn volume_round_trip(s3_endpoint: Option<String>) {
    let database = std::env::var("CITADEL_PHASE7_LOCAL_BACKUP_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database).await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&database)
        .await
        .unwrap();
    let suffix = Uuid::now_v7().simple().to_string();
    let source = format!("citadel-backup-test-{suffix}-source");
    let repository_volume = format!("citadel-backup-test-{suffix}-repository");
    let target = format!("citadel-backup-test-{suffix}-restored");
    let result = AssertUnwindSafe(async {
        for name in [&source, &repository_volume] { docker(&["volume", "create", name], None).await; }
        docker(&["run", "--rm", "-i", "--volume", &format!("{source}:/fixture"), "--entrypoint", "sh", IMAGE, "-c", "cat > /fixture/payload.txt"], Some(PAYLOAD)).await;
        let platform = Uuid::now_v7(); let secret = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
            .bind(platform).bind(format!("acceptance-{suffix}")).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')")
            .bind(secret).bind(format!("acceptance-{suffix}")).execute(&pool).await.unwrap();
        if s3_endpoint.is_some() {
            for id in [ACCESS_KEY_ID, SECRET_KEY_ID] {
                sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted') ON CONFLICT(id) DO NOTHING")
                    .bind(id).bind(format!("acceptance-credential-{id}")).execute(&pool).await.unwrap();
            }
        }
        let actor = ActorId::new(SYSTEM_ACTOR_ID);
        let store = PostgresBackupStore::new(pool.clone());
        let mut input = BackupRepositoryInput { name: format!("repo-{suffix}"), description: None, password_secret_id: secret,
            spec: json!({"$type":"FileSystem","location":"Platform","platformId":platform,"path":repository_volume}) };
        if let Some(endpoint) = &s3_endpoint {
            input.spec = json!({"$type":"S3Compatible","endpoint":endpoint,"bucket":"citadel-backups","allowInsecureHttp":true,
                "prefix":suffix,"accessKeySecretId":ACCESS_KEY_ID,"secretKeySecretId":SECRET_KEY_ID});
        }
        input.validate().unwrap();
        let repository = store.create_repository(actor, &input).await.unwrap();
        let executor = DockerResticBackupExecutor::new("docker", IMAGE, Arc::new(Password), 256 * 1024, pool.clone());
        let cancellation = CancellationToken::new();
        executor.repository(&repository, "Initialize", "Platform", Some(platform), &cancellation).await.unwrap();
        store.record_repository_operation(repository.id, "Initialize", "Platform", Some(platform), true, None).await.unwrap();
        let mut policy = BackupPolicyInput { name: format!("policy-{suffix}"), description: None,
            source: json!({"$type":"DockerVolume","platformId":platform,"volumeName":source}),
            backup_repository_id: repository.id, enabled: true, cron: None, time_zone: None, webhook: None,
            keep_last_successful: Some(2), timeout_seconds: Some(120), alert_on_failure: false,
            run_as_actor_id: None, tag_ids: vec![] };
        policy.validate(actor).unwrap();
        let policy = store.create_policy(actor, &policy).await.unwrap();
        let run = store.enqueue_backup(actor, policy.id, "Manual").await.unwrap();
        let claim = store.claim_backup(Utc::now() - chrono::Duration::hours(1)).await.unwrap().unwrap();
        assert_eq!(claim.run.id, run.id, "use a dedicated acceptance database");
        let plan = BackupSourcePlan { display_name: source.clone(), items: vec![BackupSourceItem::new(platform, source.clone(), None, None)], warnings: vec![], local_directory: None };
        store.prepare_backup_items(&claim, &plan).await.unwrap();
        let completed = executor.backup(&claim, &plan, &cancellation).await;
        store.finish_backup(&claim, &completed).await.unwrap();
        assert_eq!(completed.status, "Succeeded", "{:?}; {:?}", completed.error_message, completed.logs);
        let persisted = PostgresBackupStore::new(pool.clone()).get_run(run.id).await.unwrap();
        assert_eq!(persisted.status, "Succeeded");
        assert_eq!(persisted.items[0].restic_snapshot_id, completed.items[0].restic_snapshot_id);
        assert!(persisted.bytes_processed.unwrap() >= PAYLOAD.len() as i64);
        let queued = store.enqueue_restore(BackupRestoreRequest { actor, backup_run_id: run.id, target_platform_id: platform,
            target_volume_name: target.clone(), overwrite_existing: false, target_docker_node_id: None, source_backup_run_item_id: None }).await.unwrap();
        let restore = store.claim_restore(Utc::now() - chrono::Duration::hours(1)).await.unwrap().unwrap();
        assert_eq!(restore.run.id, queued.id);
        let restored = executor.restore(&restore, &cancellation).await;
        store.finish_restore(&restore, &restored).await.unwrap();
        assert_eq!(restored.status, "Succeeded", "{:?}", restored.error_message);
        assert_eq!(store.get_restore(queued.id).await.unwrap().status, "Succeeded");
        let bytes = docker(&["run", "--rm", "--volume", &format!("{target}:/fixture:ro"), "--entrypoint", "cat", IMAGE, "/fixture/payload.txt"], None).await;
        assert_eq!(bytes, PAYLOAD);
        // A second restore cannot overwrite an existing volume without consent.
        assert_eq!(executor.restore(&restore, &cancellation).await.status, "Failed");
    }).catch_unwind().await;
    // Only UUID-scoped fixture volumes are removed; never prune the daemon.
    for name in [&source, &repository_volume, &target] {
        let _ = run(
            ProcessRequest::new("docker")
                .args(["volume", "rm", name.as_str()])
                .limits(ProcessLimits {
                    timeout: Duration::from_secs(30),
                    ..Default::default()
                }),
            &CancellationToken::new(),
        )
        .await;
    }
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
