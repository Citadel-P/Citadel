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
        .claim_operation(actor, true, created.id, ServiceOperationKind::Apply, None)
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
    let stale_deletions = store.stale_deletion_claims(i64::MAX, 10).await.unwrap();
    assert_eq!(stale_deletions.len(), 1);
    assert_eq!(stale_deletions[0].1.id, created.id);
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
