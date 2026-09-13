use std::sync::Arc;

use citadel_adapters::crypto::AesGcmSecretProtector;
use citadel_adapters::deployment_bindings::PostgresDeploymentBindingResolver;
use citadel_adapters::deployment_store::PostgresDeploymentStore;
use citadel_database::MigrationRunner;
use citadel_deployments::{
    CreateDeploymentInput, DeploymentBindingResolverPort, DeploymentError, DeploymentImageInfo,
    DeploymentSpec, DeploymentStore, RuntimeContainerState, RuntimeDeploymentResult,
    UpdateBehavior,
};
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

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
    let store = PostgresDeploymentStore::new(pool.clone());
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

fn create_input(platform_id: Uuid, prefix: &str) -> CreateDeploymentInput {
    CreateDeploymentInput {
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

fn external_input(platform_id: Uuid, prefix: &str) -> CreateDeploymentInput {
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
    let detail = PostgresDeploymentStore::new(pool.clone())
        .get_authorized(ActorId::new(SYSTEM_ACTOR_ID), true, deployment_id)
        .await
        .unwrap();
    let latest = detail.latest_activity_view.unwrap();
    assert_eq!(latest["status"], expected_status);
    assert_eq!(
        latest
            .pointer("/info/result/message")
            .and_then(Value::as_str),
        expected_message
    );
    assert!(latest["info"].get("Result").is_none());
    assert_eq!(status, expected_status);
    assert_eq!(
        info.pointer("/Result/Message").and_then(Value::as_str),
        expected_message
    );
}
