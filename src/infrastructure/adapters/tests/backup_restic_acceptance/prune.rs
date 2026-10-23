use super::*;
use sqlx::postgres::PgPoolOptions;
use std::{collections::BTreeSet, path::Path};

// Real Restic against a unique Core repository. No production Docker or storage.
#[tokio::test]
#[ignore = "requires disposable CITADEL_LOCAL_BACKUP_DATABASE_URL and CITADEL_RESTIC_PATH"]
async fn repository_prune_preserves_all_snapshots_and_policy_retention_stays_scoped() {
    let database = std::env::var("CITADEL_LOCAL_BACKUP_DATABASE_URL").unwrap();
    let restic = std::env::var_os("CITADEL_RESTIC_PATH").unwrap();
    MigrationRunner::migrate(&database).await.unwrap();
    let pool = PgPoolOptions::new().connect(&database).await.unwrap();
    let root = std::env::temp_dir().join(format!("citadel-prune-{}", Uuid::now_v7()));
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::write(root.join("source/payload"), b"disposable fixture").unwrap();
    let result = AssertUnwindSafe(async {
        let actor = ActorId::new(SYSTEM_ACTOR_ID);
        let secret = Uuid::now_v7();
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')")
            .bind(secret).bind(format!("prune-{secret}")).execute(&pool).await.unwrap();
        // This fixture explicitly supplies shared credentials for its backup repositories.
        sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global','00000000-0000-0000-0000-000000000000') ON CONFLICT DO NOTHING").bind(secret).execute(&pool).await.unwrap();
        let store = PostgresBackupPersistence::new(pool.clone());
        let repository = store.create_repository(actor, &BackupRepositoryConfiguration {
            name: format!("prune-{}", Uuid::now_v7()), password_secret_id: secret,
            spec: serde_json::from_value(json!({"$type":"FileSystem","location":"Core","path":root.join("repository")})).unwrap(),
        }).await.unwrap();
        let executor = DockerResticBackupExecutor::new("must-not-run-docker", IMAGE, Arc::new(Password), 1024 * 1024, pool.clone()).with_restic(restic.clone());
        let cancel = CancellationToken::new();
        executor.repository(&repository, "Initialize", "Core", None, &cancel).await.unwrap();
        sqlx::query("UPDATE backuprepositories SET status='Ready' WHERE id=$1").bind(repository.id).execute(&pool).await.unwrap();
        let mut input: BackupPolicyConfiguration = serde_json::from_value(json!({
            "name":format!("retention-{}",Uuid::now_v7()), "enabled":true,
            "source":{"$type":"CitadelSystem"}, "backupRepositoryId":repository.id,
            "keepLastSuccessful":17, "timeoutSeconds":120, "alertOnFailure":false,
        })).unwrap();
        input.validate(actor).unwrap();
        let policy = store.create_policy(actor, &input).await.unwrap();
        let tag = format!("citadel-policy-{}", policy.id);
        let other_tag = format!("citadel-policy-{}", Uuid::now_v7());
        for i in 0..24 {
            let date = format!("2026-01-{:02} 12:00:00", i + 1);
            restic_command(&restic, &root, &["backup", "--tag", if i < 20 { &tag } else { &other_tag }, "--time", &date, root.join("source").to_str().unwrap()]).await;
        }
        let before = snapshots(&restic, &root).await;
        assert_eq!(before.len(), 24);
        let unrelated: BTreeSet<_> = before.iter().filter(|s| s["tags"].as_array().unwrap().contains(&json!(other_tag))).map(|s|s["id"].as_str().unwrap().to_owned()).collect();
        executor.repository(&repository, "Prune", "Core", None, &cancel).await.unwrap();
        assert_eq!(ids(&before), ids(&snapshots(&restic, &root).await), "Prune must not expire snapshots, including policies retaining more than 14");
        let run = store.enqueue_backup(actor, policy.id, "Manual", None).await.unwrap();
        let claim = store.claim_backup(Utc::now() - chrono::Duration::hours(1)).await.unwrap().unwrap();
        assert_eq!(claim.run.id, run.id);
        let result = executor.backup(&claim, &BackupSourcePlan {
            display_name: "fixture".into(), items: vec![], warnings: vec![], local_directory: Some(root.join("source")),
        }, &cancel).await;
        assert_eq!(result.status, BackupRunStatus::Succeeded, "{:?}", result.warnings);
        let after = snapshots(&restic, &root).await;
        assert_eq!(after.iter().filter(|s|s["tags"].as_array().unwrap().contains(&json!(tag))).count(), 17);
        assert!(unrelated.is_subset(&ids(&after)), "Retention must preserve unrelated policy snapshots");
        assert_eq!(after.len(), 21);
        verify_agent_prune_dispatch(&pool, &repository).await;
    }).catch_unwind().await;
    pool.close().await;
    std::fs::remove_dir_all(&root).unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

// Direct and Edge use the same Agent command constructor. Exercise that branch
// through an Edge session; Agent allowlist coverage lives in swarm_policy tests.
async fn verify_agent_prune_dispatch(pool: &sqlx::PgPool, repository: &BackupRepository) {
    use citadel_adapters::connectors::edge::{EdgeRegistry, EdgeTarget};
    use citadel_contracts::citadel::{
        containers::v1::{
            CreateContainerRequest, CreateContainerResponse, ExecBinaryRequest, ExecExit,
            ExecServerMessage, exec_server_message,
        },
        edge::v1::{EdgeCommandKind as Kind, core_envelope},
        shared_models::v1::PlatformInfoResponse,
    };
    use prost::Message;

    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'EdgeAgent',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(format!("prune-{platform}")).execute(pool).await.unwrap();
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(platform), Uuid::now_v7())
        .unwrap();
    let executor = DockerResticBackupExecutor::new(
        "docker-must-not-run",
        IMAGE,
        Arc::new(Password),
        4096,
        pool.clone(),
    )
    .with_edge(registry);
    let mut repository = repository.clone();
    repository.repository_type = "S3Compatible".into();
    repository.spec = serde_json::from_value(json!({"$type":"S3Compatible", "endpoint":"http://fixture.invalid", "bucket":"backups", "allowInsecureHttp":true,
        "accessKeySecretId":Uuid::now_v7(), "secretKeySecretId":Uuid::now_v7()})).unwrap();
    let cancellation = CancellationToken::new();
    let peer = async {
        for kind in [
            Kind::PlatformGetInfo,
            Kind::ContainerCreate,
            Kind::ContainerStart,
            Kind::ContainerExecBinary,
            Kind::ContainerDelete,
        ] {
            let envelope = tokio::time::timeout(Duration::from_secs(5), outbound.recv())
                .await
                .unwrap()
                .unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected command")
            };
            assert_eq!(command.kind, kind as i32);
            assert_eq!(command.resource_id, platform.to_string());
            match kind {
                Kind::PlatformGetInfo => session.output(
                    id,
                    PlatformInfoResponse {
                        agent_runtime_image: "citadel-agent:fixture".into(),
                        ..Default::default()
                    }
                    .encode_to_vec(),
                ),
                Kind::ContainerCreate => {
                    let request =
                        CreateContainerRequest::decode(command.payload.as_slice()).unwrap();
                    assert!(request.mounts.is_empty());
                    assert_eq!(request.image_id, "citadel-agent:fixture");
                    session.output(
                        id,
                        CreateContainerResponse {
                            container_id: "prune-helper".into(),
                        }
                        .encode_to_vec(),
                    );
                }
                Kind::ContainerExecBinary => {
                    let request = ExecBinaryRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.cmd, ["restic", "prune"]);
                    assert_eq!(request.container_id, "prune-helper");
                    assert!(request.env["RESTIC_REPOSITORY"].starts_with("s3:"));
                    assert!(!request.tty);
                    session.output(
                        id,
                        ExecServerMessage {
                            msg: Some(exec_server_message::Msg::Exit(ExecExit { exit_code: 0 })),
                        }
                        .encode_to_vec(),
                    );
                }
                _ => {}
            }
            session.complete(id, true);
        }
    };
    let (result, ()) = tokio::join!(
        executor.repository(
            &repository,
            "Prune",
            "Platform",
            Some(platform),
            &cancellation
        ),
        peer
    );
    result.unwrap();
    assert_eq!(session.pending_count(), 0);
}

async fn restic_command(restic: &std::ffi::OsStr, root: &Path, args: &[&str]) -> Vec<u8> {
    let repository = root.join("repository");
    let output = run(
        ProcessRequest::new(restic)
            .args(
                ["-r", repository.to_str().unwrap()]
                    .into_iter()
                    .chain(args.iter().copied()),
            )
            .env("RESTIC_PASSWORD", "disposable-acceptance-password"),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(
        output.succeeded(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
async fn snapshots(restic: &std::ffi::OsStr, root: &Path) -> Vec<serde_json::Value> {
    serde_json::from_slice(&restic_command(restic, root, &["snapshots", "--json"]).await).unwrap()
}
fn ids(values: &[serde_json::Value]) -> BTreeSet<String> {
    values
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect()
}
