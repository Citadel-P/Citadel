use std::sync::Arc;
use std::time::Duration;

use citadel_adapters::persistence::postgres::deployments::PostgresDeploymentRepository;
use citadel_adapters::persistence::postgres::deployments::bindings::PostgresDeploymentBindingResolver;
use citadel_adapters::persistence::postgres::platforms::runtime_index::RuntimeIdentityIndex;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_database::MigrationRunner;
use citadel_deployments::{
    CreateDeployment, DeploymentBindingResolverPort, DeploymentError, DeploymentImageInfo,
    DeploymentRepository, DeploymentSpec, RuntimeContainerState, RuntimeDeploymentResult,
    UpdateBehavior,
};
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite, SnapshotGeneration};
use citadel_primitives::ActorId;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn apply_completion_does_not_block_unrelated_inventory_on_the_same_platform() {
    let url = std::env::var("CITADEL_PHASE6_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresDeploymentRepository::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'local','Local',1,0,1048576,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(format!("deployment-concurrency-{platform}")).execute(&pool).await.unwrap();
    let mut resources = Vec::new();
    for name in ["first", "second"] {
        let resource = store
            .create(actor, true, &external_input(platform, name))
            .await
            .unwrap();
        let claim = store.claim_apply(actor, true, resource.id).await.unwrap();
        let runtime = RuntimeDeploymentResult {
            docker_container_id: format!("runtime-{}", resource.id),
            docker_image_id: "sha256:test".into(),
            state: RuntimeContainerState::Running,
        };
        store
            .complete_apply(
                actor,
                &claim,
                &runtime,
                Some("example/web@sha256:current"),
                &[],
            )
            .await
            .unwrap();
        resources.push((resource, runtime));
    }
    let (first, _) = &resources[0];
    let (second, runtime) = &resources[1];
    let claim = store.claim_apply(actor, true, second.id).await.unwrap();
    let mut inventory = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_one(&mut *inventory)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET state='Exited' WHERE deploymentid=$1")
        .bind(first.id)
        .execute(&mut *inventory)
        .await
        .unwrap();

    // Completion must finish while inventory still owns another container on
    // the same platform. A platform-wide gate would time out here.
    tokio::time::timeout(
        Duration::from_secs(5),
        store.complete_apply(actor, &claim, runtime, Some("example/web@sha256:next"), &[]),
    )
    .await
    .unwrap()
    .unwrap();
    inventory.commit().await.unwrap();
    assert_eq!(
        deployment_state(&pool, second.id).await,
        ("Healthy".into(), "Idle".into())
    );
    assert_activity(&pool, second.id, "Success", None).await;
    let digest: Option<String> =
        sqlx::query_scalar("SELECT spec->'Image'->>'ResolvedDigest' FROM deployments WHERE id=$1")
            .bind(second.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(digest.as_deref(), Some("example/web@sha256:next"));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn apply_claim_completion_failure_and_recovery_are_transactional() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = PostgresDeploymentRepository::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let platform_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'local','Local',1,0,1048576,$2,0,
                  '{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(platform_id)
    .bind(format!("phase6-apply-{}", platform_id.simple()))
    .execute(&pool)
    .await
    .unwrap();

    let index = RuntimeIdentityIndex::attach(pool.clone());
    index.rebuild().await.unwrap();
    let before_apply =
        SnapshotGeneration::capture(platform_id, None, ProjectionKind::Containers).await;
    let images = SnapshotGeneration::capture(platform_id, None, ProjectionKind::Images).await;
    let completed = store
        .create(actor, true, &external_input(platform_id, "completed"))
        .await
        .unwrap();
    let claim = store.claim_apply(actor, true, completed.id).await.unwrap();
    assert!(matches!(
        store.claim_apply(actor, true, completed.id).await,
        Err(DeploymentError::Conflict(_))
    ));
    let running = RuntimeDeploymentResult {
        docker_container_id: "docker-running".to_owned(),
        docker_image_id: "sha256:running".to_owned(),
        state: RuntimeContainerState::Running,
    };
    store
        .complete_apply(
            actor,
            &claim,
            &running,
            Some("example/web@sha256:current"),
            &[],
        )
        .await
        .unwrap();
    assert_eq!(
        deployment_state(&pool, completed.id).await,
        ("Healthy".to_owned(), "Idle".to_owned())
    );
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT deploymentid FROM containers WHERE dockercontainerid='docker-running'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        completed.id
    );
    assert!(
        !before_apply
            .matches(&ProjectionWrite::begin(platform_id, None, ProjectionKind::Containers).await)
    );
    assert!(
        images.matches(&ProjectionWrite::begin(platform_id, None, ProjectionKind::Images).await)
    );
    let persisted = index.lookup(platform_id, None, "docker-running").unwrap();
    assert_activity(&pool, completed.id, "Success", None).await;
    let applied_spec: Value = sqlx::query_scalar("SELECT spec FROM deployments WHERE id=$1")
        .bind(completed.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        applied_spec
            .pointer("/Image/ResolvedDigest")
            .and_then(Value::as_str),
        Some("example/web@sha256:current")
    );
    let activity_info: String = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid=$1 AND eventtype='DeploymentApplied' ORDER BY createdat DESC,id DESC LIMIT 1",
    )
    .bind(completed.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let activity_info: Value = serde_json::from_str(&activity_info).unwrap();
    assert_eq!(
        activity_info
            .pointer("/Deployment/Spec/Image/ResolvedDigest")
            .and_then(Value::as_str),
        Some("example/web@sha256:current")
    );

    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='DeploymentApplied'",
    )
    .bind(completed.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    store
        .complete_apply(
            actor,
            &claim,
            &running,
            Some("example/web@sha256:current"),
            &[],
        )
        .await
        .unwrap();
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='DeploymentApplied'",
    )
    .bind(completed.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        after, before,
        "a repeated completion must not duplicate audit history"
    );
    let next_claim = store.claim_apply(actor, true, completed.id).await.unwrap();
    let before_failed =
        SnapshotGeneration::capture(platform_id, None, ProjectionKind::Containers).await;
    assert!(matches!(
        store
            .complete_apply(
                actor,
                &claim,
                &running,
                Some("example/web@sha256:current"),
                &[]
            )
            .await,
        Err(DeploymentError::Conflict(_))
    ));
    assert!(
        before_failed
            .matches(&ProjectionWrite::begin(platform_id, None, ProjectionKind::Containers).await)
    );
    assert_eq!(
        index.lookup(platform_id, None, "docker-running"),
        Some(persisted)
    );
    let running = RuntimeDeploymentResult {
        docker_container_id: "docker-replaced".into(),
        ..running
    };
    store
        .complete_apply(
            actor,
            &next_claim,
            &running,
            Some("example/web@sha256:current"),
            &[],
        )
        .await
        .unwrap();

    assert_eq!(index.lookup(platform_id, None, "docker-running"), None);
    assert_eq!(
        index.lookup(platform_id, None, "docker-replaced"),
        Some(persisted)
    );
    let before_delete =
        SnapshotGeneration::capture(platform_id, None, ProjectionKind::Containers).await;
    let claims = store
        .claim_delete(actor, true, &[completed.id])
        .await
        .unwrap();
    store.complete_delete(actor, &claims).await.unwrap();
    assert_eq!(index.lookup(platform_id, None, "docker-replaced"), None);
    assert!(
        !before_delete
            .matches(&ProjectionWrite::begin(platform_id, None, ProjectionKind::Containers).await)
    );
    assert!(
        images.matches(&ProjectionWrite::begin(platform_id, None, ProjectionKind::Images).await)
    );

    let failed = store
        .create(actor, true, &create_input(platform_id, "failed"))
        .await
        .unwrap();
    let failed_claim = store.claim_apply(actor, true, failed.id).await.unwrap();
    let exited = RuntimeDeploymentResult {
        docker_container_id: "docker-exited".to_owned(),
        docker_image_id: "sha256:exited".to_owned(),
        state: RuntimeContainerState::Exited,
    };
    store
        .fail_apply(actor, &failed_claim, "container exited", Some(&exited), &[])
        .await
        .unwrap();
    assert_eq!(
        deployment_state(&pool, failed.id).await,
        ("Failed".to_owned(), "Idle".to_owned())
    );
    assert_activity(&pool, failed.id, "Failure", Some("container exited")).await;

    let stale = store
        .create(actor, true, &create_input(platform_id, "stale"))
        .await
        .unwrap();
    let stale_claim = store.claim_apply(actor, true, stale.id).await.unwrap();
    sqlx::query("UPDATE deployments SET controlstartedat=1 WHERE id=$1")
        .bind(stale.id)
        .execute(&pool)
        .await
        .unwrap();
    let stale_claims = store.stale_apply_claims(2, 10).await.unwrap();
    assert!(stale_claims.iter().any(|(_, claim)| claim.id == stale.id));
    store
        .fail_apply(actor, &stale_claim, "interrupted", None, &[])
        .await
        .unwrap();

    sqlx::query("DELETE FROM containers WHERE platformid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM deployments WHERE platformid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn binding_resolution_uses_resource_precedence_and_keeps_secrets_masked() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap();
    let deployment_id = Uuid::now_v7();
    let suffix = deployment_id.simple();
    let variable_name = format!("PHASE6_TOKEN_{suffix}");
    let secret_name = format!("PHASE6_PASSWORD_{suffix}");
    let secret_id = Uuid::now_v7();
    let global_binding_id = Uuid::now_v7();
    let resource_binding_id = Uuid::now_v7();
    let secret_binding_id = Uuid::now_v7();
    let protector = Arc::new(AesGcmSecretProtector::new(&[37_u8; 32]).unwrap());
    let envelope = protector.protect(b"sensitive-value").unwrap();

    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret_id)
    .bind(&secret_name)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue) VALUES($1,$2)")
        .bind(secret_id)
        .bind(envelope)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO resourcebindings(id,kind,name,scope,value) VALUES
               ($1,'Variable',$2,'Global','global-value')"#,
    )
    .bind(global_binding_id)
    .bind(&variable_name)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO resourcebindings(id,kind,name,resourceid,scope,value) VALUES
               ($1,'Variable',$2,$3,'Deployment','deployment-value')"#,
    )
    .bind(resource_binding_id)
    .bind(&variable_name)
    .bind(deployment_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO resourcebindings(
               id,kind,name,resourceid,scope,secretdeliverymode,secretid)
           VALUES($1,'Secret',$2,$3,'Deployment','EnvironmentVariable',$4)"#,
    )
    .bind(secret_binding_id)
    .bind(&secret_name)
    .bind(deployment_id)
    .bind(secret_id)
    .execute(&pool)
    .await
    .unwrap();

    let resolver = PostgresDeploymentBindingResolver::new(pool.clone(), protector).unwrap();
    let resolved = resolver
        .resolve(deployment_id, &[variable_name.clone(), secret_name.clone()])
        .await
        .unwrap();
    let variable = resolved
        .entries
        .iter()
        .find(|entry| entry.name == variable_name)
        .unwrap();
    let secret = resolved
        .entries
        .iter()
        .find(|entry| entry.name == secret_name)
        .unwrap();
    assert_eq!(variable.value.as_str(), "deployment-value");
    assert_eq!(variable.snapshot.value, "deployment-value");
    assert_eq!(secret.value.as_str(), "sensitive-value");
    assert_eq!(secret.snapshot.value, "********");

    sqlx::query("DELETE FROM resourcebindings WHERE id=ANY($1::uuid[])")
        .bind([global_binding_id, resource_binding_id, secret_binding_id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
        .bind(secret_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

fn create_input(platform_id: Uuid, prefix: &str) -> CreateDeployment {
    CreateDeployment {
        name: format!("{prefix}-{}", Uuid::now_v7().simple()),
        platform_id,
        description: None,
        spec: DeploymentSpec {
            image: DeploymentImageInfo::Local {
                image_id: "sha256:test".to_owned(),
            },
            update_behavior: UpdateBehavior::Disabled,
            life_cycle_spec: None,
            resource_spec: None,
            labels: None,
            ports: None,
            volumes: None,
            networks: None,
            command: None,
            environment_variables: None,
        },
        tag_ids: Vec::new(),
        duplicate_source: None,
    }
}

fn external_input(platform_id: Uuid, prefix: &str) -> CreateDeployment {
    let mut input = create_input(platform_id, prefix);
    input.spec.image = DeploymentImageInfo::External {
        registry_id: Uuid::from_u128(0x100),
        image_tag: "example/web:latest".to_owned(),
        resolved_digest: None,
    };
    input
}

async fn deployment_state(pool: &sqlx::PgPool, id: Uuid) -> (String, String) {
    sqlx::query_as::<_, (String, String)>("SELECT status,controlstate FROM deployments WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn assert_activity(
    pool: &sqlx::PgPool,
    deployment_id: Uuid,
    expected_status: &str,
    expected_message: Option<&str>,
) {
    let (status, info) = sqlx::query_as::<_, (String, String)>(
        "SELECT status,info FROM activityevents WHERE resourceid=$1 AND eventtype='DeploymentApplied' ORDER BY createdat DESC,id DESC LIMIT 1",
    )
    .bind(deployment_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let info: Value = serde_json::from_str(&info).unwrap();
    let detail = PostgresDeploymentRepository::new(pool.clone())
        .get_authorized(ActorId::new(SYSTEM_ACTOR_ID), true, deployment_id)
        .await
        .unwrap();
    let latest = detail.latest_activity.as_ref().unwrap();
    assert_eq!(latest.status.as_database_str(), expected_status);
    assert_eq!(
        latest
            .info
            .pointer("/Result/Message")
            .and_then(Value::as_str),
        expected_message
    );
    assert!(latest.info.get("Result").is_some());
    assert_eq!(status, expected_status);
    assert_eq!(
        info.pointer("/Result/Message").and_then(Value::as_str),
        expected_message
    );
}
