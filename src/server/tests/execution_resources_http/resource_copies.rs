use super::*;
use citadel_adapters::connectors::routing::platforms::runtime::PlatformRuntimeRouter;
use citadel_adapters::connectors::routing::resource_copies::CopyStorageRuntime;
use citadel_adapters::persistence::postgres::deployments::PostgresDeploymentRepository;
use citadel_adapters::persistence::postgres::resource_copies::PostgresCopyRepository;
use citadel_deployments::DeploymentRepository;
use citadel_docker::client::DockerClient;
use citadel_resource_copies::CopyService;
use citadel_server::api::routes::resource_copies::{ResourceCopiesHttpState, router};

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn copy_routes_authorize_validate_and_explain_unsupported_sources() {
    let _fixture = FIXTURE_LOCK.lock().await;
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let admin = seed_administrator(&pool).await;
    let regular = seed_regular_user(&pool).await;
    let ids = seed_dependencies(&pool, admin.actor_id).await;
    let source = PostgresDeploymentRepository::new(pool.clone())
        .create(
            admin.actor_id,
            true,
            &citadel_deployments::CreateDeployment {
                name: format!("copy-http-{}", Uuid::now_v7().simple()),
                platform_id: ids.platform,
                description: None,
                tag_ids: vec![],
                duplicate_source: None,
                spec: serde_json::from_value(
                    json!({"image":{"$type":"Local","imageId":"fixture"},"volumes":[]}),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();
    let backups = Arc::new(BackupService::new(
        Arc::new(PostgresBackupPersistence::new(pool.clone())),
        Arc::new(FakeBackupExecutor),
        Arc::new(FakeBackupPlanner::default()),
        Duration::minutes(5),
        Arc::new(AllowBackupExecution),
    ));
    // Unsupported/unauthorized previews must never contact the Docker runtime.
    let storage = CopyStorageRuntime {
        image: "restic/restic:0.18.1".into(),
        runtime: Arc::new(PlatformRuntimeRouter::new(
            pool.clone(),
            DockerClient::new("/no-copy-runtime.sock", std::time::Duration::from_secs(1)).unwrap(),
            None,
            Default::default(),
        )),
    };
    let copies = Arc::new(CopyService::new(
        Arc::new(PostgresCopyRepository::new(pool.clone())),
        Arc::new(storage),
        backups,
        Arc::new(|_, _| {}),
    ));
    let app = router(ResourceCopiesHttpState {
        copies: copies.clone(),
    });
    let targets_path = format!(
        "/api/v1/resource-copies/targets?kind=Deployment&sourceId={}",
        source.id
    );
    let response = request(&app, Method::GET, &targets_path, Some(admin.clone()), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let catalog = response_json(response).await;
    assert_eq!(catalog["sourcePlatformId"], ids.platform.to_string());
    assert_eq!(catalog["swarm"], false);
    assert!(
        catalog["platforms"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == ids.platform.to_string())
    );
    assert_eq!(
        request(
            &app,
            Method::GET,
            &targets_path,
            Some(regular.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let body =
        json!({"kind":"Deployment","sourceId":source.id,"name":"independent","selections":[]});
    for (actor, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(regular.clone()), StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            request(
                &app,
                Method::POST,
                "/api/v1/resource-copies/preview",
                actor,
                Some(body.clone())
            )
            .await
            .status(),
            expected
        );
    }
    let response = request(
        &app,
        Method::POST,
        "/api/v1/resource-copies/preview",
        Some(admin.clone()),
        Some(body.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let preview = response_json(response).await;
    assert!(
        preview["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b.as_str().unwrap().contains("No declared named volumes"))
    );
    assert!(preview.get("spec").is_none());
    let operation = preview["id"].as_str().unwrap();
    for (method, suffix) in [
        (Method::GET, ""),
        (Method::POST, "/start"),
        (Method::POST, "/cancel"),
        (Method::POST, "/cleanup"),
    ] {
        let path = format!("/api/v1/resource-copies/{operation}{suffix}");
        assert_eq!(
            request(&app, method.clone(), &path, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(&app, method, &path, Some(admin.clone()), None)
                .await
                .status(),
            StatusCode::NOT_FOUND,
            "incomplete previews must not create a durable executable review"
        );
    }
    let mut forged = body.clone();
    forged["administrator"] = json!(true);
    let mut invalid = body;
    invalid["name"] = json!("../unsafe");
    for body in [forged, invalid, json!([])] {
        assert_eq!(
            request(
                &app,
                Method::POST,
                "/api/v1/resource-copies/preview",
                Some(admin.clone()),
                Some(body)
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            &app,
            Method::GET,
            "/api/v1/resource-copies/not-a-uuid",
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    verify_eligibility(&pool, &copies, &admin, &regular, &ids, source.id).await;
    pool.close().await;
}

async fn verify_eligibility(
    pool: &sqlx::PgPool,
    copies: &CopyService,
    admin: &ActorPrincipal,
    regular: &ActorPrincipal,
    ids: &FixtureIds,
    source_id: Uuid,
) {
    use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
    use citadel_identity::{ResourceAccessInput, UserRepository};
    use citadel_primitives::{PermissionLevel, SpecificPermission};
    use citadel_resource_copies::{CopyEligibilityStatus as Status, CopyKind, CopyRepository};
    let eligibility = copies
        .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
        .await
        .unwrap();
    assert_eq!(eligibility.status, Status::Unavailable);
    assert!(eligibility.reason.unwrap().contains("No named volumes"));
    assert_eq!(
        copies
            .eligibility(regular.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap()
            .status,
        Status::Unavailable
    );
    sqlx::query(
        "UPDATE deployments SET spec=jsonb_set(spec,'{Volumes}','[\"data:/data\"]') WHERE id=$1",
    )
    .bind(source_id)
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        copies
            .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap()
            .status,
        Status::Available
    );
    verify_current_jobs(pool, copies, admin, regular, source_id).await;
    let repository = Uuid::now_v7();
    let policy = Uuid::now_v7();
    let run = Uuid::now_v7();
    sqlx::query("INSERT INTO backuprepositories(id,createdbyactorid,name,normalizedname,passwordsecretid,spec,status,type) VALUES($1,$2,$1::text,$1::text,$3,'{}','Ready','FileSystem')")
        .bind(repository).bind(admin.actor_id.value()).bind(ids.secret).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,backuprepositoryid,createdbyactorid,name,normalizedname,runasactorid,source) VALUES($1,$2,$3,$1::text,$1::text,$3,$4)")
        .bind(policy).bind(repository).bind(admin.actor_id.value()).bind(json!({"$type":"DockerVolume","platformId":ids.platform,"volumeName":"data"})).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT $1,id,backuprepositoryid,name,'FileSystem','Available',source,'Succeeded','Manual',createdbyactorid,CURRENT_TIMESTAMP FROM backuppolicies WHERE id=$2")
        .bind(run).bind(policy).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO backuprunitems(id,backuprunid,platformid,volumename,status,resticsnapshotid,completedat) VALUES($1,$2,$3,'data','Succeeded','snapshot',CURRENT_TIMESTAMP)")
        .bind(Uuid::now_v7()).bind(run).bind(ids.platform).execute(pool).await.unwrap();
    assert_eq!(
        copies
            .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap()
            .status,
        Status::Available
    );
    // No Docker endpoint exists in this fixture. Reading eligibility must only
    // consult configuration and authorized snapshot metadata, never create work.
    let reviews: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resourcecopies WHERE sourceid=$1")
        .bind(source_id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(
        reviews, 4,
        "only the explicitly started one-time copies should be persisted"
    );
    for (status, availability, expected) in [
        ("Failed", "Available", 0),
        ("Succeeded", "Expired", 0),
        ("SucceededWithWarnings", "Available", 1),
    ] {
        sqlx::query("UPDATE backupruns SET status=$2,snapshotavailability=$3 WHERE id=$1")
            .bind(run)
            .bind(status)
            .bind(availability)
            .execute(pool)
            .await
            .unwrap();
        let store = PostgresCopyRepository::new(pool.clone());
        let source = store
            .source(admin.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap();
        assert_eq!(
            store
                .snapshots(admin.actor_id, &source)
                .await
                .unwrap()
                .len(),
            expected
        );
        assert_eq!(
            copies
                .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
                .await
                .unwrap()
                .status,
            Status::Available,
            "current data does not require a usable backup"
        );
    }
    sqlx::query("UPDATE backuprepositories SET archivedat=CURRENT_TIMESTAMP WHERE id=$1")
        .bind(repository)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        copies
            .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap()
            .status,
        Status::Available
    );
    sqlx::query("UPDATE backuprepositories SET archivedat=NULL WHERE id=$1")
        .bind(repository)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE backuprunitems SET dockernodeid='wrong-node' WHERE backuprunid=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        copies
            .eligibility(admin.actor_id, CopyKind::Deployment, source_id)
            .await
            .unwrap()
            .status,
        Status::Available
    );
    sqlx::query("UPDATE backuprunitems SET dockernodeid=NULL WHERE backuprunid=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    use citadel_backups::{BackupPersistence, coverage::CoverageChangeSource};
    let mut changes = citadel_adapters::persistence::postgres::backups::coverage::changes::PostgresCoverageChanges::connect(pool).await.unwrap();
    // A policy may now point elsewhere, while its older snapshots are still
    // offered for this workload. Notify those original platforms as well.
    sqlx::query("UPDATE backuppolicies SET source=$2 WHERE id=$1")
        .bind(policy)
        .bind(json!({"$type":"CitadelSystem"}))
        .execute(pool)
        .await
        .unwrap();
    PostgresBackupPersistence::new(pool.clone())
        .update_policy_description(admin.actor_id, policy, Some("Historical snapshots changed"))
        .await
        .unwrap();
    let changed = tokio::time::timeout(std::time::Duration::from_secs(2), changes.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(changed.platform_id, ids.platform);
    assert!(changed.deployment_ids.contains(&source_id));
    let store = PostgresCopyRepository::new(pool.clone());
    let source = store
        .source(admin.actor_id, CopyKind::Deployment, source_id)
        .await
        .unwrap();
    let users = PostgresUserRepository::new(pool.clone());
    let grant = ResourceAccessInput {
        resource_type: ResourceType::BackupPolicy,
        resource_id: policy,
        permission_level: PermissionLevel::Execute,
        specific_permissions: vec![SpecificPermission::Restore],
    };
    let reader = ResourceAccessInput {
        permission_level: PermissionLevel::Read,
        specific_permissions: vec![],
        ..grant.clone()
    };
    assert!(
        store
            .snapshots(regular.actor_id, &source)
            .await
            .unwrap()
            .is_empty()
    );
    users
        .add_resource_access(
            regular.subject_id,
            &reader,
            admin.actor_id,
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert!(
        store
            .snapshots(regular.actor_id, &source)
            .await
            .unwrap()
            .is_empty(),
        "Read alone must not offer unrestorable snapshots"
    );
    users
        .remove_resource_access(
            regular.subject_id,
            &reader,
            admin.actor_id,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    users
        .add_resource_access(
            regular.subject_id,
            &grant,
            admin.actor_id,
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .snapshots(regular.actor_id, &source)
            .await
            .unwrap()
            .len(),
        1
    );
    users
        .remove_resource_access(
            regular.subject_id,
            &grant,
            admin.actor_id,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    assert!(
        store
            .snapshots(regular.actor_id, &source)
            .await
            .unwrap()
            .is_empty(),
        "revocation must invalidate cached Restore grants"
    );
    verify_restore_progress_changes(pool, admin.actor_id, source_id, repository).await;
}

async fn verify_current_jobs(
    pool: &sqlx::PgPool,
    copies: &CopyService,
    admin: &ActorPrincipal,
    regular: &ActorPrincipal,
    source_id: Uuid,
) {
    use citadel_resource_copies::*;
    let store = PostgresCopyRepository::new(pool.clone());
    let actor = admin.actor_id;
    let source = store
        .source(actor, CopyKind::Deployment, source_id)
        .await
        .unwrap();
    assert!(
        store
            .current_data_allowed(actor, source.platform_id)
            .await
            .unwrap()
    );
    assert!(
        !store
            .current_data_allowed(regular.actor_id, source.platform_id)
            .await
            .unwrap()
    );
    let policies_before: i64 = sqlx::query_scalar("SELECT count(*) FROM backuppolicies")
        .fetch_one(pool)
        .await
        .unwrap();
    for scenario in ["success", "cancel", "recover", "revoked"] {
        let mut changes = citadel_adapters::persistence::postgres::resource_copies::changes::PostgresCopyChanges::connect(pool).await.unwrap();
        let mut request = CopyRequest {
            destination_platform_id: None,
            destination_node_id: None,
            data_mode: CopyDataMode::CurrentData,
            consistency_acknowledged: false,
            kind: CopyKind::Deployment,
            source_id,
            name: format!("current-{scenario}-{}", Uuid::now_v7().simple()),
            selections: vec![],
        };
        let invalid =
            planning::plan(Uuid::now_v7(), source.clone(), request.clone(), vec![]).unwrap();
        assert!(store.save_review(actor, &invalid).await.is_err());
        request.consistency_acknowledged = true;
        let plan = planning::plan(Uuid::now_v7(), source.clone(), request, vec![]).unwrap();
        store.save_review(actor, &plan).await.unwrap();
        let id = plan.preview.id;
        let started = store.start(actor, id).await.unwrap();
        assert_eq!(next_copy_change(&mut changes).await, id);
        assert!(matches!(
            copies.get(regular.actor_id, id).await,
            Err(CopyError::NotFound)
        ));
        assert_eq!(started.data_mode, CopyDataMode::CurrentData);
        assert_eq!(started.restores.len(), 1);
        assert_eq!(
            store.start(actor, id).await.unwrap().restores[0].id,
            started.restores[0].id
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), changes.next())
                .await
                .is_err(),
            "idempotent start must not emit another change"
        );
        // start writes the current-volume plan before reserving the name. A
        // conflicting reservation must roll back both that write and its notify.
        let conflicting =
            planning::plan(Uuid::now_v7(), source.clone(), plan.request.clone(), vec![]).unwrap();
        store.save_review(actor, &conflicting).await.unwrap();
        assert!(matches!(
            store.start(actor, conflicting.preview.id).await,
            Err(CopyError::Conflict(_))
        ));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), changes.next())
                .await
                .is_err(),
            "rolled-back starts must not publish"
        );
        sqlx::query("DELETE FROM resourcecopies WHERE id=$1 AND status='Review'")
            .bind(conflicting.preview.id)
            .execute(pool)
            .await
            .unwrap();
        let (_, job) = store.claim_current(actor, id).await.unwrap().unwrap();
        assert_eq!(next_copy_change(&mut changes).await, id);
        assert_eq!(
            copies.get(actor, id).await.unwrap().restores[0].status,
            "Running"
        );
        assert!(
            store.claim_current(actor, id).await.unwrap().is_none(),
            "claimed jobs must never run twice"
        );
        match scenario {
            "success" => {
                store
                    .finish_current(actor, id, job.id, Ok(()))
                    .await
                    .unwrap();
                assert!(
                    store
                        .advance(actor, id, None)
                        .await
                        .unwrap()
                        .destination_id
                        .is_none()
                );
                assert_eq!(next_copy_change(&mut changes).await, id);
                assert!(
                    tokio::time::timeout(std::time::Duration::from_millis(50), changes.next())
                        .await
                        .is_err(),
                    "unchanged reconciliation must not publish"
                );
                let result = store.advance(actor, id, Some(Ok(()))).await.unwrap();
                assert_eq!(next_copy_change(&mut changes).await, id);
                assert_eq!(
                    copies.get(actor, id).await.unwrap().destination_id,
                    result.destination_id
                );
                assert_eq!(result.status, CopyStatus::Succeeded);
                let receipts = citadel_adapters::persistence::postgres::resource_copies::storage::destination_receipts(pool, result.destination_id.unwrap()).await.unwrap();
                assert_eq!(receipts.len(), 1);
                assert_eq!(receipts[0].restore_id, job.id);
            }
            "cancel" => {
                assert_eq!(
                    copies.cancel(actor, id).await.unwrap().status,
                    CopyStatus::Cancelling
                );
                assert_eq!(next_copy_change(&mut changes).await, id);
                store
                    .finish_current(actor, id, job.id, Ok(()))
                    .await
                    .unwrap();
                assert_eq!(
                    store.advance(actor, id, None).await.unwrap().status,
                    CopyStatus::Cancelled
                );
            }
            "recover" => {
                store.recover_current().await.unwrap();
                assert!(store.claim_current(actor, id).await.unwrap().is_none());
                // A stale completion must not overwrite the recovered failure.
                store
                    .finish_current(actor, id, job.id, Ok(()))
                    .await
                    .unwrap();
                let result = store.advance(actor, id, Some(Ok(()))).await.unwrap();
                assert_eq!(result.status, CopyStatus::Failed);
                assert!(result.destination_id.is_none());
                assert!(
                    result.restores[0]
                        .message
                        .as_ref()
                        .unwrap()
                        .contains("interrupted")
                );
            }
            "revoked" => {
                sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
                    .bind(actor.value())
                    .execute(pool)
                    .await
                    .unwrap();
                assert_eq!(
                    store.advance(actor, id, None).await.unwrap().status,
                    CopyStatus::Cancelling
                );
                store
                    .finish_current(actor, id, job.id, Ok(()))
                    .await
                    .unwrap();
                assert_eq!(
                    store.advance(actor, id, None).await.unwrap().status,
                    CopyStatus::Failed
                );
                sqlx::query("UPDATE actors SET isenabled=true WHERE id=$1")
                    .bind(actor.value())
                    .execute(pool)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
    }
    let policies_after: i64 = sqlx::query_scalar("SELECT count(*) FROM backuppolicies")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(
        policies_before, policies_after,
        "current copies must not create a backup policy"
    );
    let restores: i64 = sqlx::query_scalar("SELECT count(*) FROM resourcecopyrestores c JOIN resourcecopies p ON p.id=c.copyid WHERE p.sourceid=$1").bind(source_id).fetch_one(pool).await.unwrap();
    assert_eq!(
        restores, 0,
        "current data jobs must not enqueue policy-based restore runs"
    );
}

#[path = "resource_copies/recovery.rs"]
mod recovery;

async fn verify_restore_progress_changes(
    pool: &sqlx::PgPool,
    actor: ActorId,
    source_id: Uuid,
    repository: Uuid,
) {
    use citadel_backups::{BackupPersistence, BackupRestoreStatus, RestoreExecutionResult};
    use citadel_resource_copies::*;
    // Only persistence runs here; no storage executor accesses this fixture path.
    sqlx::query("UPDATE backuprepositories SET spec=$2 WHERE id=$1")
        .bind(repository).bind(json!({"$type":"FileSystem","location":"Core","platformId":null,"path":"/tmp/copy-notification-fixture"}))
        .execute(pool).await.unwrap();
    let store = PostgresCopyRepository::new(pool.clone());
    let source = store
        .source(actor, CopyKind::Deployment, source_id)
        .await
        .unwrap();
    let snapshots = store.snapshots(actor, &source).await.unwrap();
    let snapshot = &snapshots[0];
    let request = CopyRequest {
        destination_platform_id: None,
        destination_node_id: None,
        data_mode: CopyDataMode::ExistingBackup,
        consistency_acknowledged: false,
        kind: CopyKind::Deployment,
        source_id,
        name: format!("notification-{}", Uuid::now_v7().simple()),
        selections: vec![CopySelection {
            volume: snapshot.volume.clone(),
            backup_run_id: snapshot.backup_run_id,
            backup_run_item_id: snapshot.backup_run_item_id,
        }],
    };
    let plan = planning::plan(Uuid::now_v7(), source, request, snapshots).unwrap();
    store.save_review(actor, &plan).await.unwrap();
    let id = plan.preview.id;
    let mut changes = citadel_adapters::persistence::postgres::resource_copies::changes::PostgresCopyChanges::connect(pool).await.unwrap();
    store.start(actor, id).await.unwrap();
    assert_eq!(next_copy_change(&mut changes).await, id);
    let backups = PostgresBackupPersistence::new(pool.clone());
    let claim = backups
        .claim_restore(chrono::Utc::now() - chrono::Duration::hours(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next_copy_change(&mut changes).await, id);
    assert_eq!(
        store.get(actor, id).await.unwrap().restores[0].status,
        "Running"
    );
    let result = RestoreExecutionResult {
        status: BackupRestoreStatus::Failed,
        exit_code: Some(1),
        error_code: Some("FixtureFailure".into()),
        error_message: Some("Fixture failure".into()),
        logs: vec![],
    };
    backups.finish_restore(&claim, &result).await.unwrap();
    assert_eq!(next_copy_change(&mut changes).await, id);
    assert_eq!(
        store.get(actor, id).await.unwrap().restores[0]
            .message
            .as_deref(),
        Some("Fixture failure")
    );
    backups.finish_restore(&claim, &result).await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), changes.next())
            .await
            .is_err()
    );
    assert_eq!(
        store.advance(actor, id, None).await.unwrap().status,
        CopyStatus::Failed
    );
    assert_eq!(next_copy_change(&mut changes).await, id);
    store.cleanup(actor, id).await.unwrap();
    assert_eq!(next_copy_change(&mut changes).await, id);
    store.finish_cleanup(actor, id, Ok(())).await.unwrap();
    assert_eq!(next_copy_change(&mut changes).await, id);
    assert_eq!(
        store.get(actor, id).await.unwrap().status,
        CopyStatus::Cleaned
    );
}

async fn next_copy_change(changes: &mut impl citadel_resource_copies::CopyChangeSource) -> Uuid {
    tokio::time::timeout(std::time::Duration::from_secs(2), changes.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

#[path = "resource_copies/destinations.rs"]
mod destinations;
