#![cfg(unix)]

use chrono::Utc;
use citadel_adapters::external::backups::restic::DockerResticBackupExecutor;
use citadel_adapters::persistence::postgres::backups::PostgresBackupPersistence;
use citadel_backups::*;
use citadel_database::MigrationRunner;
use citadel_execution::{ProcessLimits, ProcessRequest};
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_processes::run;
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
#[path = "backup_restic_acceptance/multinode.rs"]
mod multinode;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Transport {
    Local,
    Agent,
    Edge,
}
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
    volume_round_trip(None, Transport::Local).await;
}

// Ports the S3/RustFS storage portion of WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode.
// Exact-node Agent routing has separate transport tests; this fixture uses Local Docker.
#[tokio::test]
#[ignore = "requires the dedicated RustFS/PostgreSQL fixture from Test-Phase7LocalBackup.ps1 -UseRustFs"]
async fn rustfs_volume_backup_restores_root_data_and_persists_real_results() {
    volume_round_trip(
        Some(
            std::env::var("CITADEL_PHASE7_RUSTFS_ENDPOINT")
                .expect("RustFS fixture endpoint required"),
        ),
        Transport::Local,
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage <published candidate>"]
async fn rustfs_agent_volume_backup_restore_and_retention_never_use_core_docker() {
    volume_round_trip(
        Some(std::env::var("CITADEL_PHASE7_RUSTFS_ENDPOINT").unwrap()),
        Transport::Agent,
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage <candidate> -UseEdgeAgent"]
async fn rustfs_edge_volume_backup_restore_and_retention_use_the_authenticated_session() {
    volume_round_trip(
        Some(std::env::var("CITADEL_PHASE7_RUSTFS_ENDPOINT").unwrap()),
        Transport::Edge,
    )
    .await;
}

async fn volume_round_trip(s3_endpoint: Option<String>, transport: Transport) {
    let use_agent = transport != Transport::Local;
    let fixture_image = if use_agent {
        std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap()
    } else {
        IMAGE.into()
    };
    // Remote helpers must use the connected Agent's installed image, without
    // depending on a separately installed Core-configured helper image.
    let helper_image = if use_agent {
        "citadel-missing-backup-helper:does-not-exist"
    } else {
        IMAGE
    };
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
    let agent_name = format!("citadel-backup-test-{suffix}-agent");
    let edge_registry = citadel_adapters::connectors::edge::EdgeRegistry::default();
    let edge_cancel = CancellationToken::new();
    let mut edge_server = None;
    let result = AssertUnwindSafe(async {
        for name in [&source, &repository_volume] { docker(&["volume", "create", name], None).await; }
        docker(&["run", "--rm", "-i", "--volume", &format!("{source}:/fixture"), "--entrypoint", "sh", &fixture_image, "-ec", "cat > /fixture/payload.txt; mkdir -p /fixture/private/nested; cp /fixture/payload.txt /fixture/private/nested/item; ln -s payload.txt /fixture/link; chown -hR 1000:1001 /fixture; chmod 750 /fixture/private /fixture/private/nested; chmod 640 /fixture/payload.txt /fixture/private/nested/item"], Some(PAYLOAD)).await;
        let platform = Uuid::now_v7(); let secret = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
            .bind(platform).bind(format!("acceptance-{suffix}")).execute(&pool).await.unwrap();
        let agent = if transport == Transport::Edge {
            use citadel_adapters::connectors::edge::EdgeIntake;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore;
            use citadel_contracts::citadel::edge::v1::edge_agent_service_server::EdgeAgentServiceServer;
            sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',status='Offline' WHERE id=$1").bind(platform).execute(&pool).await.unwrap();
            let store=PostgresEdgeStore::new(pool.clone());
            let edge_target=EdgeTarget::platform(platform);
            let (_,token,_)=store.create_enrollment(&edge_target,SYSTEM_ACTOR_ID).await.unwrap();
            let listener=tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
            let address=listener.local_addr().unwrap();
            let incoming=futures_util::stream::unfold(listener,|listener|async {Some((listener.accept().await.map(|(socket,_)|socket),listener))});
            let cancel=edge_cancel.clone();
            let intake=EdgeIntake::new(store,edge_registry.clone());
            edge_server=Some(tokio::spawn(async move {tonic::transport::Server::builder().add_service(EdgeAgentServiceServer::new(intake)).serve_with_incoming_shutdown(incoming,cancel.cancelled_owned()).await.unwrap()}));
            let image=std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
            let network=std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
            let host=std::env::var("CITADEL_PHASE7_CORE_HOST").unwrap();
            docker(&["run","--detach","--name",&agent_name,"--network",&network,"--mount","type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock","--env","CITADEL_AGENT_MODE=edge","--env",&format!("CITADEL_CORE_URL=http://{host}:{}",address.port()),"--env",&format!("CITADEL_EDGE_ENROLLMENT_TOKEN={token}"),&image],None).await;
            let deadline=tokio::time::Instant::now()+Duration::from_secs(30);
            while edge_registry.get(&edge_target).is_err() {
                assert!(tokio::time::Instant::now()<deadline,"Edge Agent did not enroll and connect");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1").bind(platform).execute(&pool).await.unwrap();
            None
        } else if use_agent {
            use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
            let image=std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
            let network=std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
            let mut key=[0;32];getrandom::fill(&mut key).unwrap();
            let signer=AgentRequestSigner::from_bytes(&key);key.fill(0);
            docker(&["run","--detach","--name",&agent_name,"--network",&network,"--mount","type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock","--env",&format!("HUB_PUBLIC_KEY={}",signer.public_key_base64()),"--env","CITADEL_AGENT_TLS_MODE=Disabled",&image],None).await;
            let address=format!("http://{agent_name}:9000");
            let deadline=tokio::time::Instant::now()+Duration::from_secs(30);
            let client=loop {
                if let Ok(client)=AgentClient::connect(&address,signer.clone(),Duration::from_secs(10),true).await
                    && client.handshake(&CancellationToken::new()).await.is_ok() {break client;}
                assert!(tokio::time::Instant::now()<deadline,"Agent backup fixture did not become ready");
                tokio::time::sleep(Duration::from_millis(250)).await;
            };
            sqlx::query("UPDATE platforms SET connectortype='Agent',address=$2 WHERE id=$1").bind(platform).bind(&address).execute(&pool).await.unwrap();
            Some(client)
        } else {None};
        if use_agent {
            use citadel_adapters::connectors::{docker::DockerClient, routing::volumes::content::VolumeContentAdapter};
            use futures_util::StreamExt;
            let cancel = CancellationToken::new();
            let browser = VolumeContentAdapter::new(
                pool.clone(),
                DockerClient::new("/no-core-docker-allowed.sock", Duration::from_secs(2)).unwrap(),
                agent.clone(),
                edge_registry.clone(),
                "citadel-missing-volume-helper:does-not-exist".into(),
                citadel_runtime::DynamicTasks::new(cancel.clone()),
            );
            let listing = browser.list(platform, &source, "/", None, &cancel).await.unwrap();
            assert!(listing.entries.iter().any(|entry| entry.name == "payload.txt"));
            let mut download = browser.download(platform, &source, "/payload.txt", None, &cancel).await.unwrap();
            let mut bytes = Vec::new();
            while let Some(chunk) = download.stream.next().await { bytes.extend(chunk.unwrap()); }
            assert_eq!(bytes, PAYLOAD);
            let helpers = docker(&["ps", "--all", "--quiet", "--filter", &format!("label=com.citadel.platform-id={platform}"), "--filter", "label=citadel.volume-browser=true"], None).await;
            assert!(helpers.is_empty(), "Volume browser helpers must be cleaned up");
        }
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')")
            .bind(secret).bind(format!("acceptance-{suffix}")).execute(&pool).await.unwrap();
        if s3_endpoint.is_some() {
            for id in [ACCESS_KEY_ID, SECRET_KEY_ID] {
                sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted') ON CONFLICT(id) DO NOTHING")
                    .bind(id).bind(format!("acceptance-credential-{id}")).execute(&pool).await.unwrap();
            }
        }
        let actor = ActorId::new(SYSTEM_ACTOR_ID);
        let store = PostgresBackupPersistence::new(pool.clone());
        let mut input = BackupRepositoryConfiguration { name: format!("repo-{suffix}"), description: None, password_secret_id: secret,
            spec: serde_json::from_value(json!({"$type":"FileSystem","location":"Platform","platformId":platform,"path":repository_volume})).unwrap() };
        if let Some(endpoint) = &s3_endpoint {
            input.spec = serde_json::from_value(json!({"$type":"S3Compatible","endpoint":endpoint,"bucket":"citadel-backups","allowInsecureHttp":true,
                "prefix":suffix,"accessKeySecretId":ACCESS_KEY_ID,"secretKeySecretId":SECRET_KEY_ID})).unwrap();
        }
        input.validate().unwrap();
        let repository = store.create_repository(actor, &input).await.unwrap();
        let executor = DockerResticBackupExecutor::new(if use_agent {"/no-core-docker-allowed"} else {"docker"}, helper_image, Arc::new(Password), 256 * 1024, pool.clone()).with_agent(agent).with_edge(edge_registry.clone());
        let cancellation = CancellationToken::new();
        executor.repository(&repository, "Initialize", "Platform", Some(platform), &cancellation).await.unwrap();
        let operation_id=Uuid::now_v7();
        assert!(store.acquire_repository_operation(repository.id, operation_id, "Initialize", Utc::now()+chrono::Duration::minutes(5)).await.unwrap());
        store.record_repository_operation(repository.id, citadel_backups::BackupRepositoryOperation { operation_id, operation: "Initialize", location: "Platform", platform_id: Some(platform), succeeded: true, message: None }).await.unwrap();
        store.release_repository_operation(repository.id, operation_id).await.unwrap();
        let mut policy = BackupPolicyConfiguration { name: format!("policy-{suffix}"), description: None,
            source: serde_json::from_value(json!({"$type":"DockerVolume","platformId":platform,"volumeName":source})).unwrap(),
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
        assert_eq!(completed.status, citadel_backups::BackupRunStatus::Succeeded, "{:?}; {:?}", completed.error_message, completed.logs);
        let persisted = PostgresBackupPersistence::new(pool.clone()).get_run(run.id).await.unwrap();
        assert_eq!(persisted.status, citadel_backups::BackupRunStatus::Succeeded);
        assert_eq!(persisted.items[0].restic_snapshot_id, completed.items[0].restic_snapshot_id);
        assert!(persisted.bytes_processed.unwrap() >= PAYLOAD.len() as i64);
        let queued = store.enqueue_restore(BackupRestoreRequest { actor, backup_run_id: run.id, target_platform_id: platform,
            target_volume_name: target.clone(), overwrite_existing: false, target_docker_node_id: None, source_backup_run_item_id: None }).await.unwrap();
        let restore = store.claim_restore(Utc::now() - chrono::Duration::hours(1)).await.unwrap().unwrap();
        assert_eq!(restore.run.id, queued.id);
        let restored = executor.restore(&restore, &cancellation).await;
        store.finish_restore(&restore, &restored).await.unwrap();
        assert_eq!(restored.status, citadel_backups::BackupRestoreStatus::Succeeded, "{:?}", restored.error_message);
        assert_eq!(store.get_restore(queued.id).await.unwrap().status, citadel_backups::BackupRestoreStatus::Succeeded);
        let bytes = docker(&["run", "--rm", "--volume", &format!("{target}:/fixture:ro"), "--entrypoint", "cat", &fixture_image, "/fixture/payload.txt"], None).await;
        assert_eq!(bytes, PAYLOAD);
        let metadata = docker(&["run", "--rm", "--volume", &format!("{target}:/fixture:ro"), "--entrypoint", "sh", &fixture_image, "-ec", "stat -c '%u:%g:%a' /fixture/payload.txt /fixture/private /fixture/private/nested /fixture/private/nested/item /fixture/link; readlink /fixture/link; cmp /fixture/payload.txt /fixture/private/nested/item"], None).await;
        assert_eq!(String::from_utf8(metadata).unwrap(), "1000:1001:640\n1000:1001:750\n1000:1001:750\n1000:1001:640\n1000:1001:777\npayload.txt\n", "Restores must preserve ownership, permissions and symlinks");
        // A second restore cannot overwrite an existing volume without consent.
        assert_eq!(executor.restore(&restore, &cancellation).await.status, citadel_backups::BackupRestoreStatus::Failed);
    }).catch_unwind().await;
    if use_agent {
        if result.is_err()
            && let Ok(logs) = run(
                ProcessRequest::new("docker").args(["logs", "--tail", "30", &agent_name]),
                &CancellationToken::new(),
            )
            .await
        {
            eprintln!(
                "Agent fixture output: {}\n{}",
                String::from_utf8_lossy(&logs.stdout),
                String::from_utf8_lossy(&logs.stderr)
            );
        }
        let _ = run(
            ProcessRequest::new("docker")
                .args(["rm", "--force", "--volumes", &agent_name])
                .limits(ProcessLimits {
                    timeout: Duration::from_secs(30),
                    ..Default::default()
                }),
            &CancellationToken::new(),
        )
        .await;
    }
    edge_cancel.cancel();
    if let Some(server) = edge_server {
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .unwrap()
            .unwrap();
    }
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
