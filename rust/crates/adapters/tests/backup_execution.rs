use chrono::{Duration, Timelike, Utc};
use citadel_adapters::backup_authorization::IdentityBackupRunAuthorizer;
use citadel_adapters::backup_executor::{DockerResticBackupExecutor, PostgresBackupSecretResolver};
use citadel_adapters::backup_source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::backup_store::PostgresBackupStore;
use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_backups::{
    BackupExecutionResult, BackupExecutor, BackupPolicyInput, BackupRepositoryInput,
    BackupRunAuthorizer, BackupRunItemResult, BackupSourceItem, BackupSourcePlan,
    BackupSourcePlanner, BackupStore,
};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::{
    ADMIN_ROLE_ID, IdentityService, NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn backup_claims_are_repository_exclusive_and_late_results_do_not_overwrite_recovery() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let secret = Uuid::now_v7();
    let platform = Uuid::now_v7();
    let tag = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,$5)")
        .bind(user)
        .bind(actor.value())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{user}@example.test"))
        .bind(format!("backup-user-{}", user.simple()))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#445566',$3)")
        .bind(tag)
        .bind(format!("backup-tag-{}", tag.simple()))
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("backup-secret-{}", secret.simple()))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(platform)
        .bind(format!("backup-platform-{}", platform.simple()))
        .execute(&pool)
        .await
        .unwrap();

    let store = PostgresBackupStore::new(pool.clone());
    let mut repository_input = BackupRepositoryInput {
        name: format!("backup-repository-{}", Uuid::now_v7().simple()),
        description: None,
        spec: json!({"$type":"FileSystem","location":"Core","platformId":null,"path":"/tmp/citadel-backups"}),
        password_secret_id: secret,
    };
    repository_input.validate().unwrap();
    let repository = store
        .create_repository(actor, &repository_input)
        .await
        .unwrap();
    let (_, validation) = store
        .record_repository_operation(repository.id, "Validate", "Core", None, true, None)
        .await
        .unwrap();
    assert_eq!(validation.status, "Ready");
    let persisted_validation: (String, String) =
        sqlx::query_as("SELECT location,status FROM backuprepositoryvalidations WHERE id=$1")
            .bind(validation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(persisted_validation, ("Core".into(), "Ready".into()));

    let mut policies = Vec::new();
    for suffix in ["one", "two"] {
        let mut input = BackupPolicyInput {
            name: format!("backup-policy-{suffix}-{}", Uuid::now_v7().simple()),
            description: None,
            source: json!({"$type":"DockerVolume","platformId":platform,"volumeName":format!("volume-{suffix}")}),
            backup_repository_id: repository.id,
            enabled: true,
            cron: None,
            time_zone: None,
            webhook: None,
            keep_last_successful: Some(2),
            timeout_seconds: Some(60),
            alert_on_failure: true,
            run_as_actor_id: None,
            tag_ids: vec![tag],
        };
        input.validate(actor).unwrap();
        policies.push(store.create_policy(actor, &input).await.unwrap());
    }
    let persisted_tags: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resourcetags WHERE resourcetype='BackupPolicy' AND resourceid=ANY($1) AND tagid=$2",
    )
    .bind(policies.iter().map(|policy| policy.id).collect::<Vec<_>>())
    .bind(tag)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted_tags, 2);
    let mut invalid_policy = BackupPolicyInput {
        name: format!("invalid-tag-policy-{}", Uuid::now_v7().simple()),
        description: None,
        source: json!({"$type":"DockerVolume","platformId":platform,"volumeName":"invalid"}),
        backup_repository_id: repository.id,
        enabled: true,
        cron: None,
        time_zone: None,
        webhook: None,
        keep_last_successful: Some(2),
        timeout_seconds: Some(60),
        alert_on_failure: true,
        run_as_actor_id: None,
        tag_ids: vec![Uuid::now_v7()],
    };
    invalid_policy.validate(actor).unwrap();
    assert!(matches!(
        store.create_policy(actor, &invalid_policy).await,
        Err(citadel_backups::BackupError::Validation(_))
    ));
    let rolled_back: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuppolicies WHERE normalizedname=$1)")
            .bind(invalid_policy.name.to_uppercase())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!rolled_back);
    let first = store
        .enqueue_backup(actor, policies[0].id, "Manual")
        .await
        .unwrap();
    let _second = store
        .enqueue_backup(actor, policies[1].id, "Manual")
        .await
        .unwrap();
    assert_eq!(first.snapshot_availability, "Pending");

    let (left, right) = tokio::join!(
        store.claim_backup(Utc::now() - Duration::minutes(5)),
        store.claim_backup(Utc::now() - Duration::minutes(5))
    );
    let claims = [left.unwrap(), right.unwrap()];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    let first_claim = claims.into_iter().flatten().next().unwrap();
    let guarded_claim = first_claim.clone();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[71_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let authorizer = IdentityBackupRunAuthorizer::new(identity);
    assert!(authorizer.authorize_backup(&guarded_claim).await.is_err());
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.value())
        .bind(ADMIN_ROLE_ID)
        .execute(&pool)
        .await
        .unwrap();
    authorizer.authorize_backup(&guarded_claim).await.unwrap();
    let planner = PostgresBackupSourcePlanner::new(pool.clone());
    let plan = planner
        .plan(&first_claim, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(plan.items.len(), 1);
    store
        .prepare_backup_items(&first_claim, &plan)
        .await
        .unwrap();
    let prepared = store.get_run(first_claim.run.id).await.unwrap();
    assert_eq!(prepared.items.len(), 1);
    assert_eq!(prepared.items[0].status, "Queued");
    store
        .finish_backup(&first_claim, &success_result("a", Some(plan.items[0].id)))
        .await
        .unwrap();
    let completed = store.get_run(first_claim.run.id).await.unwrap();
    assert_eq!(completed.items[0].status, "Succeeded");

    let late_claim = store
        .claim_backup(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    sqlx::query(
        "UPDATE backupruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP WHERE id=$1",
    )
    .bind(late_claim.run.id)
    .execute(&pool)
    .await
    .unwrap();
    store
        .finish_backup(&late_claim, &success_result("b", None))
        .await
        .unwrap();
    assert_eq!(
        store.get_run(late_claim.run.id).await.unwrap().status,
        "Interrupted"
    );
    assert!(
        store
            .backup_logs(late_claim.run.id)
            .await
            .unwrap()
            .is_empty()
    );

    sqlx::query("UPDATE backuppolicies SET cron='* * * * *',timezone='UTC',controlstate='Idle',currentrunid=NULL WHERE id=$1")
        .bind(policies[0].id)
        .execute(&pool)
        .await
        .unwrap();
    let scheduled_minute = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let (left, right) = tokio::join!(
        store.enqueue_scheduled_backup(policies[0].id, scheduled_minute),
        store.enqueue_scheduled_backup(policies[0].id, scheduled_minute)
    );
    assert_eq!(usize::from(left.unwrap()) + usize::from(right.unwrap()), 1);
    let scheduled_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backupruns WHERE backuppolicyid=$1 AND trigger='Schedule'",
    )
    .bind(policies[0].id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scheduled_count, 1);

    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let executor = DockerResticBackupExecutor::new(
        "docker-command-must-not-run",
        "restic/restic:test",
        Arc::new(
            PostgresBackupSecretResolver::new(
                pool.clone(),
                Arc::new(AesGcmSecretProtector::new(&[92_u8; 32]).unwrap()),
            )
            .unwrap(),
        ),
        4096,
        pool.clone(),
    );
    let rejected = executor
        .backup(
            &guarded_claim,
            &BackupSourcePlan {
                display_name: "fixture".into(),
                items: vec![BackupSourceItem::new(
                    platform,
                    "volume-one".into(),
                    None,
                    None,
                )],
                warnings: vec![],
                local_directory: None,
            },
            &CancellationToken::new(),
        )
        .await;
    assert_eq!(rejected.status, "Failed");
    assert!(
        rejected
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("Edge Agent is disconnected or unavailable"))
    );
    edge_failure_cleans_helper_on_the_exact_node(executor, &guarded_claim, platform).await;
    edge_restore_uses_the_saved_snapshot_root(&pool, &guarded_claim, platform).await;

    sqlx::query("DELETE FROM backuprunlogs WHERE backuprunid IN (SELECT id FROM backupruns WHERE backuprepositoryid=$1)").bind(repository.id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backupruns WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM resourcetags WHERE resourcetype='BackupPolicy' AND resourceid=ANY($1)",
    )
    .bind(policies.iter().map(|policy| policy.id).collect::<Vec<_>>())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM backuppolicies WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(tag)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuprepositories WHERE id=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
        .bind(secret)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actorroles WHERE actorid=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}

async fn edge_failure_cleans_helper_on_the_exact_node(
    executor: DockerResticBackupExecutor,
    claim: &citadel_backups::BackupClaim,
    platform: Uuid,
) {
    use citadel_adapters::edge::{EdgeRegistry, EdgeTarget};
    use citadel_contracts::citadel::{
        containers::v1::CreateContainerResponse,
        edge::v1::{EdgeCommandKind, core_envelope},
    };
    use prost::Message;
    let registry = EdgeRegistry::default();
    let (_wrong, mut wrong_outbound) = registry
        .register(
            EdgeTarget::node(platform, "node-one".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (selected, mut outbound) = registry
        .register(
            EdgeTarget::node(platform, "node-two".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let executor = executor.with_edge(registry);
    let plan = BackupSourcePlan {
        display_name: "fixture".into(),
        items: vec![BackupSourceItem::new(
            platform,
            "volume-one".into(),
            Some("node-two".into()),
            None,
        )],
        warnings: vec![],
        local_directory: None,
    };
    let cancellation = CancellationToken::new();
    let run = executor.backup(claim, &plan, &cancellation);
    let agent = async {
        for kind in [
            EdgeCommandKind::ContainerCreate,
            EdgeCommandKind::ContainerStart,
            EdgeCommandKind::ContainerDelete,
        ] {
            let message = tokio::time::timeout(std::time::Duration::from_secs(5), outbound.recv())
                .await
                .unwrap()
                .unwrap();
            let id = Uuid::parse_str(&message.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = message.body else {
                panic!("expected command")
            };
            assert_eq!(command.node_id, "node-two");
            assert_eq!(command.kind, kind as i32);
            if kind == EdgeCommandKind::ContainerCreate {
                selected.output(
                    id,
                    CreateContainerResponse {
                        container_id: "helper-on-node-two".into(),
                    }
                    .encode_to_vec(),
                );
            }
            selected.complete(id, kind != EdgeCommandKind::ContainerStart);
        }
    };
    let (result, ()) = tokio::join!(run, agent);
    assert_eq!(result.status, "Failed");
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].status, "Failed");
    assert!(wrong_outbound.try_recv().is_err());
    assert_eq!(selected.pending_count(), 0);
}

// Extends .NET BackupRestoreRunExecutionTests with cross-connector root parity.
// The peer validates canonical Agent commands, not a real Restic installation.
async fn edge_restore_uses_the_saved_snapshot_root(
    pool: &sqlx::PgPool,
    backup: &citadel_backups::BackupClaim,
    platform: Uuid,
) {
    use citadel_adapters::edge::{EdgeRegistry, EdgeTarget};
    use citadel_backups::{BackupRestoreRunView, BackupSecretResolver, RestoreClaim};
    use citadel_contracts::citadel::{
        containers::v1::{
            CreateContainerRequest, CreateContainerResponse, ExecBinaryRequest, ExecExit,
            ExecOutput, ExecServerMessage, exec_server_message,
        },
        edge::v1::{EdgeCommandKind as Kind, core_envelope},
        volumes::v1::ListVolumesResponse,
    };
    use futures_util::future::BoxFuture;
    use prost::Message;
    use zeroize::Zeroizing;
    struct Password;
    impl BackupSecretResolver for Password {
        fn resolve(
            &self,
            _: Uuid,
        ) -> BoxFuture<'_, Result<Zeroizing<String>, citadel_backups::BackupError>> {
            Box::pin(async { Ok(Zeroizing::new("fixture-password".into())) })
        }
    }
    for root in ["/data", "/source", "/unexpected"] {
        let registry = EdgeRegistry::default();
        let (_wrong, mut other) = registry
            .register(
                EdgeTarget::node(platform, "source-node".into()),
                Uuid::now_v7(),
            )
            .unwrap();
        let (session, mut receiver) = registry
            .register(
                EdgeTarget::node(platform, "restore-node".into()),
                Uuid::now_v7(),
            )
            .unwrap();
        let executor = DockerResticBackupExecutor::new(
            "docker-must-not-run",
            "restic:test",
            Arc::new(Password),
            4096,
            pool.clone(),
        )
        .with_edge(registry);
        let snapshot = "a".repeat(64);
        let mut repository = backup.repository.clone();
        repository.repository_type = "S3Compatible".into();
        repository.spec = json!({"endpoint":"http://s3.test", "bucket":"backups",
            "accessKeySecretId":Uuid::now_v7(), "secretKeySecretId":Uuid::now_v7()});
        let mut source = backup.run.clone();
        source.restic_snapshot_id = Some(snapshot.clone());
        let claim = RestoreClaim {
            repository,
            source,
            source_item: None,
            run: BackupRestoreRunView {
                id: Uuid::now_v7(),
                backup_run_id: backup.run.id,
                backup_repository_id: backup.repository.id,
                source_backup_run_item_id: None,
                target_platform_id: platform,
                target_docker_node_id: Some("restore-node".into()),
                target_volume_name: "target".into(),
                overwrite_existing: false,
                status: "Running".into(),
                queued_at: Utc::now(),
                started_at: Some(Utc::now()),
                completed_at: None,
                exit_code: None,
                error_code: None,
                error_message: None,
                triggered_by_actor_id: SYSTEM_ACTOR_ID,
            },
        };
        let cancellation = CancellationToken::new();
        let peer = async {
            let mut kinds = vec![
                Kind::VolumeList,
                Kind::ContainerCreate,
                Kind::ContainerStart,
                Kind::ContainerExecBinary,
            ];
            if root != "/unexpected" {
                kinds.push(Kind::ContainerExecBinary);
            }
            kinds.push(Kind::ContainerDelete);
            let mut metadata = true;
            for kind in kinds {
                let envelope =
                    tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv())
                        .await
                        .unwrap()
                        .unwrap();
                let id = Uuid::parse_str(&envelope.command_id).unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected command");
                };
                assert_eq!(command.kind, kind as i32);
                assert_eq!(command.node_id, "restore-node");
                match kind {
                    Kind::VolumeList => {
                        session.output(id, ListVolumesResponse::default().encode_to_vec());
                    }
                    Kind::ContainerCreate => {
                        let request =
                            CreateContainerRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.mounts[0].source.as_deref(), Some("target"));
                        assert_eq!(request.mounts[0].target.as_deref(), Some("/target"));
                        session.output(
                            id,
                            CreateContainerResponse {
                                container_id: "restore-helper".into(),
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::ContainerExecBinary => {
                        let request =
                            ExecBinaryRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.container_id, "restore-helper");
                        assert_eq!(request.env["RESTIC_PASSWORD"], "fixture-password");
                        if metadata {
                            assert_eq!(request.cmd, ["restic", "snapshots", "--json", &snapshot]);
                            session.output(
                                id,
                                ExecServerMessage {
                                    msg: Some(exec_server_message::Msg::Output(ExecOutput {
                                        data: serde_json::to_vec(
                                            &json!([{"id":snapshot, "paths":[root]}]),
                                        )
                                        .unwrap(),
                                        stream: 0,
                                    })),
                                }
                                .encode_to_vec(),
                            );
                        } else {
                            assert_eq!(
                                request.cmd,
                                [
                                    "restic",
                                    "restore",
                                    &format!("{snapshot}:{root}"),
                                    "--target",
                                    "/target",
                                    "--delete"
                                ]
                            );
                        }
                        metadata = false;
                        session.output(
                            id,
                            ExecServerMessage {
                                msg: Some(exec_server_message::Msg::Exit(ExecExit {
                                    exit_code: 0,
                                })),
                            }
                            .encode_to_vec(),
                        );
                    }
                    _ => {}
                }
                session.complete(id, true);
            }
        };
        let (result, ()) = tokio::join!(executor.restore(&claim, &cancellation), peer);
        assert_eq!(
            result.status,
            if root == "/unexpected" {
                "Failed"
            } else {
                "Succeeded"
            }
        );
        assert!(
            other.try_recv().is_err(),
            "the source node must not receive restore commands"
        );
        assert_eq!(session.pending_count(), 0);
    }
}

fn success_result(marker: &str, item_id: Option<Uuid>) -> BackupExecutionResult {
    let snapshot = marker.repeat(64);
    BackupExecutionResult {
        status: "Succeeded",
        snapshot_availability: "Available",
        restic_snapshot_id: Some(snapshot.clone()),
        parent_snapshot_id: None,
        files_processed: Some(1),
        bytes_processed: Some(1),
        bytes_added: Some(1),
        exit_code: Some(0),
        error_code: None,
        error_message: None,
        warnings: vec![],
        logs: vec![],
        items: item_id
            .map(|id| BackupRunItemResult {
                id,
                status: "Succeeded",
                restic_snapshot_id: Some(snapshot),
                parent_snapshot_id: None,
                files_processed: Some(1),
                bytes_processed: Some(1),
                bytes_added: Some(1),
                exit_code: Some(0),
                error_code: None,
                error_message: None,
            })
            .into_iter()
            .collect(),
    }
}
