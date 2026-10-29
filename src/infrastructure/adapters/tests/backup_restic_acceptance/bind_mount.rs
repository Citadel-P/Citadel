use super::*;
use citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner;
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
#[ignore = "requires dedicated CITADEL_LOCAL_BACKUP_DATABASE_URL, Docker and restic/restic:0.18.1"]
async fn bind_mount_directory_and_file_round_trip_with_overwrite_guards() {
    let database = std::env::var("CITADEL_LOCAL_BACKUP_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database).await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&database)
        .await
        .unwrap();
    let suffix = Uuid::now_v7().simple().to_string();
    let root = std::env::temp_dir().join(format!("citadel-bind-{suffix}"));
    std::fs::create_dir_all(root.join("source/private")).unwrap();
    std::fs::create_dir(root.join("target")).unwrap();
    std::fs::create_dir(root.join("workload-target")).unwrap();
    std::fs::create_dir(root.join("stack-target")).unwrap();
    std::fs::create_dir(root.join("excluded,path")).unwrap();
    std::fs::write(root.join("source/private/payload.txt"), PAYLOAD).unwrap();
    std::fs::set_permissions(
        root.join("source/private/payload.txt"),
        std::fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    std::os::unix::fs::symlink("private/payload.txt", root.join("source/link")).unwrap();
    std::fs::write(root.join("file-target"), b"original").unwrap();
    let workload_container = format!("citadel-bind-workload-{suffix}");
    let workload_volume = format!("citadel-bind-data-{suffix}");
    let repository_volume = format!("citadel-bind-repo-{suffix}");
    let result = AssertUnwindSafe(async {
        docker(&["volume", "create", &repository_volume], None).await;
        let platform = Uuid::now_v7();
        let secret = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(platform).bind(format!("bind-{suffix}")).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')")
            .bind(secret).bind(format!("bind-{suffix}")).execute(&pool).await.unwrap();
        // This fixture explicitly supplies shared credentials for its backup repositories.
        sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global','00000000-0000-0000-0000-000000000000') ON CONFLICT DO NOTHING").bind(secret).execute(&pool).await.unwrap();
        let actor = ActorId::new(SYSTEM_ACTOR_ID);
        let store = PostgresBackupPersistence::new(pool.clone());
        let repository = store.create_repository(actor, &BackupRepositoryConfiguration {
            name: format!("bind-repo-{suffix}"), password_secret_id: secret,
            spec: serde_json::from_value(json!({"$type":"FileSystem","location":"Platform","platformId":platform,"path":repository_volume})).unwrap(),
        }).await.unwrap();
        let executor = DockerResticBackupExecutor::new("docker", IMAGE, Arc::new(Password), 256 * 1024, pool.clone());
        let cancel = CancellationToken::new();
        repository_operation(&store, &executor, &repository, "Initialize", "Platform", Some(platform), &cancel).await.unwrap();
        let operation_id = Uuid::now_v7();
        assert!(store.acquire_repository_operation(repository.id, operation_id, "Initialize", Utc::now()+chrono::Duration::minutes(5)).await.unwrap());
        store.record_repository_operation(repository.id, BackupRepositoryOperation { operation_id, operation: "Initialize", location: "Platform", platform_id: Some(platform), succeeded: true, message: None }).await.unwrap();
        store.release_repository_operation(repository.id, operation_id).await.unwrap();
        let deployment = Uuid::now_v7();
        sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$2,$3,$4,$5,'Stopped')")
            .bind(deployment).bind(format!("mixed-{suffix}")).bind(platform).bind(actor.value())
            .bind(serde_json::from_value::<citadel_deployments::DeploymentSpec>(json!({"image":{"$type":"Local","imageId":IMAGE},"volumes":[format!("{workload_volume}:/volume"), format!("{}:/bind", root.join("source").display())]})).unwrap().to_storage_value().unwrap())
            .execute(&pool).await.unwrap();
        docker(&["volume", "create", &workload_volume], None).await;
        // The named volume is resolved from the saved Deployment specification.
        // Keep it detached here: Docker Desktop can expose internal /run paths
        // when inspecting a container with both named volumes and bind mounts.
        let container_id = String::from_utf8(docker(&["create", "--name", &workload_container,
            "--label", "com.docker.compose.service=app",
            "--volume", &format!("{}:/bind", root.join("source").display()),
            "--volume", &format!("{}:/config.txt", root.join("source/private/payload.txt").display()),
            "--volume", &format!("{}:/excluded:ro", root.join("excluded,path").display()),
            IMAGE], None).await).unwrap().trim().to_owned();
        sqlx::query("INSERT INTO containers(id,created,updated,deploymentid,dockercontainerid,dockerimageid,name,platformid,ports,state) VALUES($1,0,0,$2,$3,'image',$4,$5,'{}','Stopped')")
            .bind(Uuid::now_v7()).bind(deployment).bind(&container_id).bind(&workload_container).bind(platform).execute(&pool).await.unwrap();
        use citadel_stacks::{CreateStack, StackRepository, StackSource, StackSpec, StackSpecCommon, StackUpdateBehavior};
        let stack = citadel_adapters::persistence::postgres::stacks::PostgresStackRepository::new(pool.clone()).create(actor, true, &CreateStack {
            name: format!("bind-stack-{suffix}"), platform_id: platform, description: None, stack_source: StackSource::WebEditor,
            spec: StackSpec::WebEditor { compose_file: format!("services:\n  app:\n    image: {IMAGE}\n    volumes:\n      - {}:/bind\n", root.join("source").display()), update_behavior: StackUpdateBehavior::Disabled, common: StackSpecCommon::default() },
            drift_policy: None, tag_ids: vec![], duplicate_source: None,
        }).await.unwrap();
        for (name, source, target, overwrite) in [
            ("workload", root.join("source"), root.join("workload-target"), false),
            ("stack", root.join("source"), root.join("stack-target"), false),
            ("directory", root.join("source"), root.join("target"), false),
            ("file", root.join("source/private/payload.txt"), root.join("file-target"), true),
        ] {
            if name == "stack" {
                sqlx::query("UPDATE containers SET deploymentid=NULL,stackid=$1 WHERE dockercontainerid=$2 AND platformid=$3")
                    .bind(stack.id).bind(&container_id).bind(platform).execute(&pool).await.unwrap();
            }
            let mut configuration = BackupPolicyConfiguration {
                name: format!("bind-{name}-{suffix}"), description: None,
                source: if name == "stack" { spec::BackupSourceSpec::Stack { stack_id: stack.id } } else if name == "workload" { spec::BackupSourceSpec::Deployment { deployment_id: deployment } } else { spec::BackupSourceSpec::BindMount { platform_id: platform, host_path: source.display().to_string(), docker_node_id: None } },
                backup_repository_id: repository.id, enabled: true, cron: None, time_zone: None, webhook: None,
                keep_last_successful: Some(2), timeout_seconds: Some(120), alert_on_failure: false, run_as_actor_id: None, tag_ids: vec![],
            };
            configuration.validate(actor).unwrap();
            let policy = store.create_policy(actor, &configuration).await.unwrap();
            let run = store.enqueue_backup(actor, policy.id, "Manual", None).await.unwrap();
            let claim = store.claim_backup(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();
            use citadel_docker::client::DockerClient;
use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::routing::containers::ContainerRuntimeRouter;
            let runtime = ContainerRuntimeRouter::new(pool.clone(), DockerClient::new("/var/run/docker.sock", Duration::from_secs(30)).unwrap(), None, EdgeRegistry::default());
            let planner = PostgresBackupSourcePlanner::new(pool.clone()).with_runtime(runtime);
            let plan = planner.plan(&claim, &cancel).await.unwrap();
            assert!(plan.items.iter().any(|item| item.host_path.as_deref() == source.to_str()), "{name}: items={:?}, warnings={:?}", plan.items, plan.warnings);
            if name == "workload" {
                assert_eq!(plan.items.len(), 3, "One volume and two overlapping bind paths must coexist");
                let preview = planner.preview(policies::read_models::BackupPreviewKind::Deployment, deployment, actor, true, &cancel).await.unwrap();
                assert_eq!(preview.volumes.iter().filter(|item| item.kind == "BindMount").count(), 2);
                // A saved volume must not hide missing container inventory.
                sqlx::query("UPDATE containers SET deploymentid=NULL WHERE dockercontainerid=$1 AND platformid=$2")
                    .bind(&container_id).bind(platform).execute(&pool).await.unwrap();
                assert!(matches!(planner.plan(&claim, &cancel).await,
                    Err(BackupError::Validation(message)) if message.contains("No synchronized containers")));
                sqlx::query("UPDATE containers SET deploymentid=$1 WHERE dockercontainerid=$2 AND platformid=$3")
                    .bind(deployment).bind(&container_id).bind(platform).execute(&pool).await.unwrap();
            }
            if name == "stack" {
                assert_eq!(plan.items.len(), 2, "A bind-only Stack must have eligible sources without volume bindings");
                let preview = planner.preview(policies::read_models::BackupPreviewKind::Stack, stack.id, actor, true, &cancel).await.unwrap();
                assert_eq!(preview.volumes.iter().filter(|item| item.kind == "BindMount").count(), 2);
                // One healthy Container must not mask another owned Container's
                // stale inventory. Preview keeps known paths and reports uncertainty.
                let stale_container = Uuid::now_v7();
                sqlx::query("INSERT INTO containers(id,created,updated,stackid,dockercontainerid,dockerimageid,name,platformid,ports,state,projectionstalesince) VALUES($1,0,0,$2,$3,'image',$3,$4,'{}','Running',1)")
                    .bind(stale_container).bind(stack.id).bind(format!("stale-{suffix}"))
                    .bind(platform).execute(&pool).await.unwrap();
                assert!(matches!(planner.plan(&claim, &cancel).await,
                    Err(BackupError::Validation(message)) if message.contains("stale or incomplete inventory")));
                let partial = planner.preview(policies::read_models::BackupPreviewKind::Stack, stack.id, actor, true, &cancel).await.unwrap();
                assert_eq!(partial.volumes.iter().filter(|item| item.kind == "BindMount").count(), 2);
                assert!(partial.warnings.iter().any(|warning| warning.contains("stale or incomplete inventory")));
                sqlx::query("UPDATE containers SET projectionstalesince=1 WHERE dockercontainerid=$1 AND platformid=$2")
                    .bind(&container_id).bind(platform).execute(&pool).await.unwrap();
                assert!(matches!(planner.plan(&claim, &cancel).await,
                    Err(BackupError::Validation(message)) if message.contains("stale or incomplete inventory")));
                let stale = planner.preview(policies::read_models::BackupPreviewKind::Stack, stack.id, actor, true, &cancel).await.unwrap();
                assert!(stale.volumes.iter().all(|item| item.kind != "BindMount"));
                assert!(stale.warnings.iter().any(|warning| warning.contains("stale or incomplete inventory")));
                sqlx::query("DELETE FROM containers WHERE id=$1").bind(stale_container).execute(&pool).await.unwrap();
                sqlx::query("UPDATE containers SET projectionstalesince=NULL WHERE dockercontainerid=$1 AND platformid=$2")
                    .bind(&container_id).bind(platform).execute(&pool).await.unwrap();
                assert_eq!(planner.plan(&claim, &cancel).await.unwrap().items.len(), 2);
            }
            store.prepare_backup_items(&claim, &plan).await.unwrap();
            let backed = executor.backup(&claim, &plan, &cancel).await;
            store.finish_backup(&claim, &backed).await.unwrap();
            let expected_status = if name == "workload" || name == "stack" {
                assert!(backed.warnings.iter().any(|warning| warning.contains("excluded,path")));
                BackupRunStatus::SucceededWithWarnings
            } else { BackupRunStatus::Succeeded };
            assert_eq!(backed.status, expected_status, "{:?}", backed.error_message);
            let persisted = store.get_run(run.id).await.unwrap();
            let selected = persisted.items.iter().find(|item| item.host_path.as_deref() == source.to_str()).unwrap();
            let request = BackupRestoreRequest { actor, backup_run_id: run.id, target_platform_id: platform, target_volume_name: String::new(), target_host_path: Some(target.display().to_string()), overwrite_existing: overwrite, target_docker_node_id: None, source_backup_run_item_id: Some(selected.id) };
            store.enqueue_restore(request.clone()).await.unwrap();
            let restore = store.claim_restore(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();
            assert_eq!(restore.run.target_host_path.as_deref(), target.to_str());
            let restored = executor.restore(&restore, &cancel).await;
            store.finish_restore(&restore, &restored).await.unwrap();
            assert_eq!(restored.status, BackupRestoreStatus::Succeeded, "{:?}", restored.error_message);
            let output = if name != "file" { target.join("private/payload.txt") } else { target.clone() };
            assert_eq!(std::fs::read(&output).unwrap(), PAYLOAD);
            assert_eq!(std::fs::metadata(&output).unwrap().permissions().mode() & 0o777, 0o640);
            if name != "file" { assert_eq!(std::fs::read_link(target.join("link")).unwrap(), std::path::Path::new("private/payload.txt")); }
            let mut guarded = restore.clone();
            guarded.run.overwrite_existing = false;
            assert_eq!(executor.restore(&guarded, &cancel).await.status, BackupRestoreStatus::Failed);
            assert_eq!(std::fs::read(output).unwrap(), PAYLOAD);
            let missing = root.join("does-not-exist");
            guarded.run.target_host_path = Some(missing.display().to_string());
            assert_eq!(executor.restore(&guarded, &cancel).await.status, BackupRestoreStatus::Failed);
            assert!(!missing.exists(), "A misspelled bind target must not be created");
            // Allowing warning-status runs must preserve per-snapshot safeguards.
            for (item_status, snapshot) in [("Failed", selected.restic_snapshot_id.clone()), ("Succeeded", None)] {
                sqlx::query("UPDATE backuprunitems SET status=$2,resticsnapshotid=$3 WHERE id=$1")
                    .bind(selected.id).bind(item_status).bind(snapshot).execute(&pool).await.unwrap();
                assert!(matches!(store.enqueue_restore(request.clone()).await, Err(BackupError::Conflict(_))));
            }
            sqlx::query("UPDATE backuprunitems SET status='Succeeded',resticsnapshotid=$2 WHERE id=$1")
                .bind(selected.id).bind(&selected.restic_snapshot_id).execute(&pool).await.unwrap();
            sqlx::query("UPDATE backupruns SET snapshotavailability='Expired' WHERE id=$1")
                .bind(run.id).execute(&pool).await.unwrap();
            assert!(matches!(store.enqueue_restore(request.clone()).await, Err(BackupError::Conflict(_))));
            sqlx::query("UPDATE backupruns SET snapshotavailability='Available',status='Failed' WHERE id=$1")
                .bind(run.id).execute(&pool).await.unwrap();
            assert!(matches!(store.enqueue_restore(request.clone()).await, Err(BackupError::Conflict(_))));
            sqlx::query("UPDATE backupruns SET status=$2 WHERE id=$1")
                .bind(run.id).bind(expected_status.as_str()).execute(&pool).await.unwrap();
            let mut wrong_type = request;
            wrong_type.target_host_path = None;
            wrong_type.target_volume_name = "not-a-host-path".into();
            assert!(store.enqueue_restore(wrong_type).await.is_err());
        }
    }).catch_unwind().await;
    let _ = run(
        ProcessRequest::new("docker").args(["rm", "--force", "--volumes", &workload_container]),
        &CancellationToken::new(),
    )
    .await;
    let _ = run(
        ProcessRequest::new("docker").args(["volume", "rm", "--force", &workload_volume]),
        &CancellationToken::new(),
    )
    .await;
    let _ = run(
        ProcessRequest::new("docker").args(["volume", "rm", "--force", &repository_volume]),
        &CancellationToken::new(),
    )
    .await;
    let _ = std::fs::remove_dir_all(root);
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
