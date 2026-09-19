use std::collections::BTreeMap;

use citadel_adapters::swarm_service_store::PostgresSwarmServiceStore;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_swarm_services::{
    CreateSwarmServiceInput, RuntimeServiceResult, SchedulingMode, ServiceOperationKind,
    SwarmServiceFilter, SwarmServiceImageInfo, SwarmServiceSpec, SwarmServiceStore, UpdateBehavior,
    UpdateSwarmServiceInput,
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn managed_swarm_service_crud_projection_and_operation_state_are_persisted() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = PostgresSwarmServiceStore::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let suffix = Uuid::now_v7().simple().to_string();
    let platform_id = Uuid::now_v7();
    let registry_id = Uuid::now_v7();

    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,
                  '{"$type":"DockerSwarm","clusterId":"fixture","controlAvailable":true}'::json,
                  'Online',0)"#,
    )
    .bind(platform_id)
    .bind(format!("swarm-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}'::json,$2,$3,'docker.io','Enabled')",
    )
    .bind(registry_id)
    .bind(SYSTEM_ACTOR_ID)
    .bind(format!("registry-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();

    let created = store
        .create(
            actor,
            true,
            &CreateSwarmServiceInput {
                name: format!("redis-{suffix}"),
                platform_id,
                description: Some("fixture".to_owned()),
                spec: spec(registry_id, 1),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(created.health, "Created");
    assert!(created.has_pending_desired_changes);
    assert_eq!(created.tasks.as_ref().unwrap().len(), 0);

    let stale = store
        .update(
            actor,
            true,
            created.id,
            &UpdateSwarmServiceInput {
                spec: spec(registry_id, 2),
                row_version: created.row_version + 1,
            },
        )
        .await;
    assert!(stale.is_err());

    let updated = store
        .update(
            actor,
            true,
            created.id,
            &UpdateSwarmServiceInput {
                spec: spec(registry_id, 2),
                row_version: created.row_version,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.spec.replicas, Some(2));

    let claim = store
        .claim_operation(
            actor,
            true,
            citadel_swarm_services::ServiceOperationRequest {
                id: created.id,
                kind: ServiceOperationKind::Apply,
                replicas: None,
                expected_version: None,
            },
        )
        .await
        .unwrap();
    store.mark_attempted(&claim).await.unwrap();
    let accepted = RuntimeServiceResult {
        docker_service_id: format!("docker-{suffix}"),
        version_index: 4,
        accepted: true,
        rollout_complete: false,
        rollout_error: None,
        runtime_hash: claim.desired_hash.clone(),
        applied_digest: Some("sha256:fixture".to_owned()),
        warnings: Vec::new(),
    };
    store.mark_accepted(&claim, &accepted).await.unwrap();
    assert_eq!(
        store
            .stale_operation_claims(i64::MAX, 10)
            .await
            .unwrap()
            .len(),
        1
    );

    let completed = RuntimeServiceResult {
        rollout_complete: true,
        ..accepted
    };
    store
        .complete_operation(actor, &claim, &completed)
        .await
        .unwrap();
    let current = store.get_authorized(actor, true, created.id).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT health FROM swarmservices WHERE id=$1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Healthy"
    );
    assert_eq!(current.health, "Unknown");
    assert!(!current.has_pending_desired_changes);
    assert_eq!(current.current_operation.unwrap().state, "Completed");
    assert_eq!(
        store
            .list_authorized(
                actor,
                true,
                &SwarmServiceFilter {
                    platform_id: Some(platform_id),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .len(),
        1
    );

    let claims = store.delete(actor, true, &[created.id]).await.unwrap();
    store.mark_delete_attempted(&claims[0]).await.unwrap();
    let stale_deletions = store.stale_deletion_claims(i64::MAX, 10).await.unwrap();
    assert!(
        stale_deletions.is_empty(),
        "dispatched deletes are reconciled from inventory, not replayed"
    );
    store.complete_delete(actor, &claims).await.unwrap();
    assert!(store.get_authorized(actor, true, created.id).await.is_err());
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM registries WHERE id=$1")
        .bind(registry_id)
        .execute(&pool)
        .await
        .unwrap();
}

fn spec(registry_id: Uuid, replicas: i32) -> SwarmServiceSpec {
    SwarmServiceSpec {
        image: SwarmServiceImageInfo::External {
            registry_id,
            image_tag: "redis:7-alpine".to_owned(),
            resolved_digest: None,
        },
        update_behavior: UpdateBehavior::Disabled,
        scheduling_mode: SchedulingMode::Replicated,
        replicas: Some(replicas),
        command: Vec::new(),
        arguments: Vec::new(),
        environment: Vec::new(),
        labels: BTreeMap::new(),
        user: None,
        working_directory: None,
        health_check: None,
        stop_grace_period_nanoseconds: None,
        ports: Vec::new(),
        network_ids: Vec::new(),
        mounts: Vec::new(),
        secrets: Vec::new(),
        configs: Vec::new(),
        resources: None,
        placement_constraints: Vec::new(),
        restart_policy: None,
        update_policy: None,
        webhook: None,
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn authoritative_swarm_observation_completes_or_fails_operations_once() {
    use citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore;
    use citadel_platforms::{
        InventoryProjectionStore, RuntimeInventorySnapshot, RuntimePlatformInfo, RuntimeSwarmInfo,
        RuntimeSwarmInventory, RuntimeSwarmService,
    };
    let url = std::env::var("CITADEL_PHASE6_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresSwarmServiceStore::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let platform = Uuid::now_v7();
    let registry = Uuid::now_v7();
    let cluster = format!("cluster-{platform}");
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount,clusterid) VALUES($1,'unix:///fixture','Local',1,0,1024,$2,0,$3,'Online',0,$4)")
        .bind(platform).bind(format!("platform-{platform}")).bind(serde_json::json!({"$type":"DockerSwarm","clusterId":cluster,"controlAvailable":true})).bind(&cluster).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}',$2,$3,'docker.io','Enabled')")
        .bind(registry).bind(SYSTEM_ACTOR_ID).bind(format!("registry-{registry}")).execute(&pool).await.unwrap();
    let created = store
        .create(
            actor,
            true,
            &CreateSwarmServiceInput {
                name: format!("service-{}", Uuid::now_v7()),
                platform_id: platform,
                description: None,
                spec: spec(registry, 1),
                tag_ids: vec![],
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    let claim = store
        .claim_operation(
            actor,
            true,
            citadel_swarm_services::ServiceOperationRequest {
                id: created.id,
                kind: ServiceOperationKind::Apply,
                replicas: None,
                expected_version: None,
            },
        )
        .await
        .unwrap();
    store.mark_attempted(&claim).await.unwrap();
    sqlx::query("UPDATE swarmservices SET targetruntimehash='target',preparedat=now()-interval '3 minutes',attemptedat=now()-interval '3 minutes' WHERE id=$1").bind(created.id).execute(&pool).await.unwrap();
    let service = RuntimeSwarmService {
        id: "runtime-service".into(),
        version_index: 4,
        desired_task_count: 1,
        running_task_count: 1,
        update_state: "Updating".into(),
        runtime_hash: "target".into(),
        labels: BTreeMap::from([
            ("com.citadel.managed".into(), "true".into()),
            ("com.citadel.service-id".into(), created.id.to_string()),
            (
                "com.citadel.operation-id".into(),
                claim.operation_id.to_string(),
            ),
        ]),
        ..Default::default()
    }
    .normalize_ownership();
    let mut snapshot = RuntimeInventorySnapshot {
        platform_id: platform,
        observed_at: chrono::Utc::now(),
        info: RuntimePlatformInfo {
            daemon_id: format!("daemon-{platform}"),
            swarm: Some(RuntimeSwarmInfo {
                cluster_id: Some(cluster),
                node_id: "manager".into(),
                local_node_state: "active".into(),
                control_available: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        containers: vec![],
        images: vec![],
        networks: vec![],
        volumes: vec![],
        swarm: Some(RuntimeSwarmInventory {
            services: vec![service],
            ..Default::default()
        }),
    };
    let inventory = PostgresInventoryProjectionStore::new(pool.clone());
    async fn operation(pool: &sqlx::PgPool, id: Uuid) -> (String, String) {
        sqlx::query_as("SELECT operationstate,controlstate FROM swarmservices WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }
    inventory.persist(&snapshot).await.unwrap();
    assert_eq!(
        operation(&pool, created.id).await,
        ("Accepted".into(), "Processing".into())
    );
    snapshot.swarm.as_mut().unwrap().services[0].update_state = "Completed".into();
    snapshot.observed_at = chrono::Utc::now();
    inventory.persist(&snapshot).await.unwrap();
    assert_eq!(
        operation(&pool, created.id).await,
        ("Completed".into(), "Idle".into())
    );
    let count: i64=sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceApplied'").bind(created.id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    inventory.persist(&snapshot).await.unwrap();
    let count: i64=sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceApplied'").bind(created.id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        count, 1,
        "repeated snapshots cannot duplicate success activity"
    );
    let next = store
        .claim_operation(
            actor,
            true,
            citadel_swarm_services::ServiceOperationRequest {
                id: created.id,
                kind: ServiceOperationKind::Apply,
                replicas: None,
                expected_version: None,
            },
        )
        .await
        .unwrap();
    store.mark_attempted(&next).await.unwrap();
    store
        .fail_operation(actor, &next, "transport disconnected", true)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmservices SET preparedat=now()-interval '4 minutes',attemptedat=now()-interval '4 minutes',completedat=now()-interval '1 minute' WHERE id=$1").bind(created.id).execute(&pool).await.unwrap();
    snapshot.observed_at = chrono::Utc::now();
    inventory.persist(&snapshot).await.unwrap();
    assert_eq!(
        operation(&pool, created.id).await,
        ("NotAccepted".into(), "Idle".into()),
        "unchanged runtime proves ambiguous update was not accepted"
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceOperationFailed'").bind(created.id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    let deletion = store.delete(actor, true, &[created.id]).await.unwrap();
    store.mark_delete_attempted(&deletion[0]).await.unwrap();
    sqlx::query("UPDATE swarmservices SET preparedat=now()-interval '3 minutes' WHERE id=$1")
        .bind(created.id)
        .execute(&pool)
        .await
        .unwrap();
    snapshot.swarm.as_mut().unwrap().services.clear();
    snapshot.observed_at = chrono::Utc::now();
    inventory.persist(&snapshot).await.unwrap();
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmservices WHERE id=$1)")
        .bind(created.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        !exists,
        "authoritative absence completes dispatched delete without replaying it"
    );
    store.complete_delete(actor, &deletion).await.unwrap();
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceDeleted'").bind(created.id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn image_scanner_deduplicates_resources_and_excludes_unsupported_swarm_workloads() {
    use citadel_adapters::{
        image_digest_cache::ImageDigestCache,
        image_scanner::{ImageScanRuntime, ImageScanTask, ImageScanner},
    };
    use citadel_deployments::DeploymentRepository;
    use citadel_stacks::StackStore;
    use std::sync::{Arc, Mutex};
    use tokio_util::sync::CancellationToken;
    #[derive(Default)]
    struct Inspector(Mutex<Vec<String>>);
    impl ImageScanRuntime for Inspector {
        fn inspect<'a>(
            &'a self,
            task: &'a ImageScanTask,
            _: &'a CancellationToken,
        ) -> futures_util::future::BoxFuture<'a, Result<String, String>> {
            self.0.lock().unwrap().push(task.reference.clone());
            Box::pin(async { Ok("sha256:remote".into()) })
        }
    }
    let url = std::env::var("CITADEL_PHASE6_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let registry = Uuid::now_v7();
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///'||$2,'Local',1,0,1024,$2,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).bind(format!("scanner-{platform}")).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}',$2,$3,'docker.io','Enabled')").bind(registry).bind(SYSTEM_ACTOR_ID).bind(format!("registry-{registry}")).execute(&pool).await.unwrap();
    let input = citadel_deployments::CreateDeployment {
        name: format!("deployment-{platform}"),
        platform_id: platform,
        description: None,
        spec: serde_json::from_value(serde_json::json!({"image":{"$type":"External","registryId":registry,"imageTag":"nginx:latest"},"updateBehavior":"Notify"})).unwrap(),
        tag_ids: Vec::new(),
        duplicate_source: None,
    };
    let deployment =
        citadel_adapters::postgres::deployments::PostgresDeploymentRepository::new(pool.clone())
            .create(actor, true, &input)
            .await
            .unwrap();
    let input=serde_json::from_value(serde_json::json!({"name":format!("stack-{platform}"),"platformId":platform,"stackSource":"WebEditor","spec":{"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:latest\n","registryId":registry,"updateBehavior":"Notify"}})).unwrap();
    let stack = citadel_adapters::stack_store::PostgresStackStore::new(pool.clone())
        .create(actor, true, &input)
        .await
        .unwrap();
    sqlx::query("UPDATE stackreleases SET status='Healthy' WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .execute(&pool)
        .await
        .unwrap();
    let inspector = Arc::new(Inspector::default());
    let cache = Arc::new(ImageDigestCache::default());
    let scanner = ImageScanner::new(pool.clone(), inspector.clone(), cache.clone());
    assert_eq!(
        scanner.run_cycle(&CancellationToken::new()).await.unwrap(),
        1
    );
    assert_eq!(*inspector.0.lock().unwrap(), vec!["nginx:latest"]);
    assert_eq!(
        cache.get(registry, "nginx").unwrap().digest,
        "sha256:remote"
    );
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"clusterId\":\"scanner\",\"controlAvailable\":true}' WHERE id=$1").bind(platform).execute(&pool).await.unwrap();
    inspector.0.lock().unwrap().clear();
    assert_eq!(
        scanner.run_cycle(&CancellationToken::new()).await.unwrap(),
        0,
        "Deployment and Compose image checks are unsupported on Swarm"
    );
    let mut definition = spec(registry, 1);
    definition.update_behavior = UpdateBehavior::Notify;
    let service = PostgresSwarmServiceStore::new(pool.clone())
        .create(
            actor,
            true,
            &CreateSwarmServiceInput {
                name: format!("managed-{platform}"),
                platform_id: platform,
                description: None,
                spec: definition,
                tag_ids: vec![],
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        scanner.run_cycle(&CancellationToken::new()).await.unwrap(),
        1
    );
    assert_eq!(*inspector.0.lock().unwrap(), vec!["redis:7-alpine"]);
    sqlx::query("UPDATE swarmservices SET appliedimagedigest='sha256:current',health='Healthy',lastapplieddesiredspechash=desiredspechash WHERE id=$1").bind(service.id).execute(&pool).await.unwrap();
    let router = Arc::new(
        citadel_adapters::swarm_service_runtime::SwarmServiceRuntimeRouter::new(
            pool.clone(),
            citadel_adapters::docker::DockerClient::new(
                "/no-docker-for-cached-check",
                std::time::Duration::from_millis(100),
            )
            .unwrap(),
            None,
        )
        .with_image_cache(cache.clone()),
    );
    let checker = citadel_swarm_services::ManagedSwarmServiceService::new(
        Arc::new(PostgresSwarmServiceStore::new(pool.clone())),
        router.clone(),
        CancellationToken::new(),
    )
    .with_image_digests(router);
    checker
        .run_scheduled_update_checks(&CancellationToken::new())
        .await
        .unwrap();
    let status: String =
        sqlx::query_scalar("SELECT autoupdatestate_status FROM swarmservices WHERE id=$1")
            .bind(service.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "UpdateAvailable",
        "cached Notify check works without an automation entitlement or Docker request"
    );
    sqlx::query("DELETE FROM swarmservices WHERE id=$1")
        .bind(service.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM stacks WHERE id=$1")
        .bind(stack.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(deployment.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=ANY($1)")
        .bind(vec![service.id, stack.id, deployment.id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM registries WHERE id=$1")
        .bind(registry)
        .execute(&pool)
        .await
        .unwrap();
}
