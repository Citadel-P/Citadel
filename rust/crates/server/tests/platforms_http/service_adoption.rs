use super::*;
use citadel_adapters::{
    connectors::routing::swarm_services::SwarmServiceRuntimeRouter,
    persistence::postgres::swarm_services::{
        PostgresSwarmServiceAdoption, PostgresSwarmServiceRepository,
    },
};
use citadel_server::api::routes::{
    swarm_services as swarm_services_http, swarm_services::SwarmServicesHttpState,
};
use citadel_swarm_services::SwarmServiceService;
use tokio_util::sync::CancellationToken;

async fn setup() -> (Fixture, Arc<Mutex<super::swarm_inventory::DockerState>>) {
    let (mut f, runtime) = super::swarm_inventory::fixture_swarm().await;
    let router = SwarmServiceRuntimeRouter::new(f.pool.clone(), f.docker.clone(), None);
    let services = SwarmServiceService::new(
        Arc::new(
            citadel_server::tasks::swarm_services::TrackedSwarmServiceTasks::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
        Arc::new(PostgresSwarmServiceRepository::new(f.pool.clone())),
        Arc::new(router.clone()),
        CancellationToken::new(),
    )
    .with_adoption(Arc::new(PostgresSwarmServiceAdoption::new(
        f.pool.clone(),
        router,
        &[29; 32],
    )));
    f.app = f
        .app
        .merge(swarm_services_http::router(SwarmServicesHttpState {
            identity: f.lookup_state.platforms.identity.clone(),
            services: Arc::new(services),
        }));
    (f, runtime)
}
fn url(f: &Fixture, suffix: &str) -> String {
    format!(
        "/api/v1/platforms/{}/swarm/services/service-1/{suffix}",
        f.platform_id
    )
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn service_adoption_rejects_a_different_manager_identity() {
    let (f, runtime) = setup().await;
    sqlx::query("UPDATE platforms SET platformdescriptor=jsonb_set(platformdescriptor::jsonb,'{daemonId}','\"different-daemon\"') WHERE id=$1")
        .bind(f.platform_id).execute(&f.pool).await.unwrap();
    status(
        send(
            &f,
            &url(&f, "adoption-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert!(runtime.lock().await.mutations.is_empty());
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn service_adoption_requires_access_to_the_selected_registry() {
    use citadel_primitives::{ResourceType, SpecificPermission};
    let (f, runtime) = setup().await;
    let actor = super::lookup::subject(&f).await;
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
        .bind(role)
        .bind(format!("service-adopter-{role}"))
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.actor_id.value())
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,$3,2,0)")
        .bind(Uuid::now_v7()).bind(role).bind(ResourceType::SwarmService as i32).execute(&f.pool).await.unwrap();
    super::lookup::grant(
        &f,
        actor.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        SpecificPermission::Inspect as i32,
    )
    .await;
    let draft = status(
        send(&f, &url(&f, "adoption-draft"), Some(actor.clone())).await,
        StatusCode::OK,
    )
    .await;
    let mut body = input(&draft);
    body["tagIds"] = Value::Null;
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            actor.clone(),
            body.clone(),
        )
        .await,
        StatusCode::NOT_FOUND,
    )
    .await;
    assert!(runtime.lock().await.mutations.is_empty());
    super::lookup::grant(
        &f,
        actor.actor_id.value(),
        ResourceType::Registry,
        Uuid::parse_str("00000000-0000-0000-0000-000000000100").unwrap(),
        0,
    )
    .await;
    status(
        send_json(&f, Method::POST, &url(&f, "adopt"), actor, body).await,
        StatusCode::OK,
    )
    .await;
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}
async fn status(response: axum::response::Response, expected: StatusCode) -> Value {
    let actual = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(actual, expected, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}
fn input(draft: &Value) -> Value {
    let mut spec = draft["draft"]["spec"].clone();
    spec["image"]["registryId"] = json!("00000000-0000-0000-0000-000000000100");
    spec["environment"] = json!(["MODE=production"]);
    json!({"name":draft["draft"]["name"],"description":draft["draft"]["description"],"spec":spec,"previewFingerprint":draft["previewFingerprint"]})
}

// ManagedSwarmServiceEndpointTests: AdoptEndpoints_ShouldClaimUnmanagedServiceWithoutMutatingDocker,
// DuplicateDraft_ShouldCreateServiceWithoutImageProvenance_AndRecordDuplicateActivity.
#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn service_adoption_and_duplication_are_atomic_and_do_not_mutate_docker() {
    let (f, runtime) = setup().await;
    runtime.lock().await.services.get_mut("service-1").unwrap()["Spec"]["Mode"]["Replicated"]["Replicas"] =
        json!(0);
    let draft = status(
        send(
            &f,
            &url(&f, "adoption-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        draft["draft"]["spec"]["environment"],
        json!(["TOKEN=********"])
    );
    assert!(!draft.to_string().contains("do-not-return"));
    assert_eq!(draft["source"]["dockerServiceId"], "service-1");
    let mut body = input(&draft);
    body["spec"]["replicas"] = json!(1);
    body["tagIds"] = json!([f.tag_id]);
    let adopted = status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            body,
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let id = adopted["id"].as_str().unwrap();
    assert_eq!(
        adopted["health"], "Stopped",
        "observed zero-replica Service must not appear healthy"
    );
    assert_eq!(adopted["synchronizationState"], "DesiredChangesPending");
    assert_eq!(adopted["hasPendingDesiredChanges"], true);
    assert_eq!(adopted["dockerServiceId"], "service-1");
    assert_eq!(adopted["dockerName"], "redis");
    assert!(adopted["appliedImageDigest"].is_null());
    assert!(runtime.lock().await.mutations.is_empty());
    let inspected = status(
        send(
            &f,
            &format!("/api/v1/swarmServices/{id}/inspect"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(inspected["id"], "service-1");
    assert!(inspected.get("definition").is_none());
    assert!(!inspected.to_string().contains("do-not-return"));
    sqlx::query("INSERT INTO resourcebindings(id,kind,name,resourceid,scope,value) VALUES($1,'Variable','MODE',$2,'SwarmService','production')")
        .bind(Uuid::now_v7()).bind(Uuid::parse_str(id).unwrap()).execute(&f.pool).await.unwrap();
    let duplicate = status(
        send(
            &f,
            &format!("/api/v1/swarmServices/{id}/duplicate-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let duplicate = &duplicate["draft"];
    assert_eq!(duplicate["name"], "redis-copy");
    assert_eq!(duplicate["tagIds"], json!([f.tag_id]));
    assert!(duplicate["spec"]["webhook"].is_null());
    let created=status(send_json(&f,Method::POST,"/api/v1/swarmServices",f.administrator.clone(),json!({
        "name":duplicate["name"],"platformId":f.platform_id,"description":duplicate["description"],"spec":duplicate["spec"],"tagIds":duplicate["tagIds"],
        "duplicateSource":{"resourceId":id,"resourceType":"SwarmService","resourceName":"untrusted-name"}
    })).await,StatusCode::OK).await;
    assert!(created["dockerServiceId"].is_null());
    let copied: String = sqlx::query_scalar("SELECT value FROM resourcebindings WHERE scope='SwarmService' AND resourceid=$1 AND name='MODE'")
        .bind(Uuid::parse_str(created["id"].as_str().unwrap()).unwrap()).fetch_one(&f.pool).await.unwrap();
    assert_eq!(copied, "production");
    assert_eq!(created["health"], "Created");
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT eventtype FROM activityevents WHERE resourceid=$1 OR resourceid=$2",
    )
    .bind(Uuid::parse_str(id).unwrap())
    .bind(Uuid::parse_str(created["id"].as_str().unwrap()).unwrap())
    .fetch_all(&f.pool)
    .await
    .unwrap();
    assert!(events.iter().any(|e| e == "SwarmServiceAdopted"));
    assert!(events.iter().any(|e| e == "SwarmServiceDuplicated"));
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            input(&draft),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn service_adoption_rejects_unauthorized_stale_tampered_and_conflicting_sources() {
    let (f, runtime) = setup().await;
    status(
        send(&f, &url(&f, "adoption-draft"), None).await,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    let denied = super::lookup::subject(&f).await;
    status(
        send(&f, &url(&f, "adoption-draft"), Some(denied)).await,
        StatusCode::FORBIDDEN,
    )
    .await;
    let draft = status(
        send(
            &f,
            &url(&f, "adoption-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let mut body = input(&draft);
    body["spec"]["image"]["imageTag"] = json!("nginx");
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            body,
        )
        .await,
        StatusCode::BAD_REQUEST,
    )
    .await;
    let mut body = input(&draft);
    body["spec"]["environment"] = draft["draft"]["spec"]["environment"].clone();
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            body,
        )
        .await,
        StatusCode::BAD_REQUEST,
    )
    .await;
    let mut body = input(&draft);
    body["previewFingerprint"] = json!("0".repeat(64));
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            body,
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    runtime.lock().await.services.get_mut("service-1").unwrap()["Version"]["Index"] = json!(2);
    status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "adopt"),
            f.administrator.clone(),
            input(&draft),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    runtime.lock().await.services.get_mut("service-1").unwrap()["Spec"]["Labels"] =
        json!({"com.docker.stack.namespace":"stack"});
    status(
        send(
            &f,
            &url(&f, "adoption-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert!(runtime.lock().await.mutations.is_empty());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM swarmservices WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    // AdoptEndpoints_ShouldReclaimOrphanedServiceAndRemoveStaleOwnershipLabels.
    let labels = json!({"com.citadel.managed":"true","com.citadel.service-id":Uuid::now_v7(),"team":"platform"});
    runtime.lock().await.services.get_mut("service-1").unwrap()["Spec"]["Labels"] = labels.clone();
    sqlx::query("UPDATE swarmserviceprojections SET ownership='CitadelService',labels=$2 WHERE platformid=$1")
        .bind(f.platform_id).bind(&labels).execute(&f.pool).await.unwrap();
    let orphan = status(
        send(
            &f,
            &url(&f, "adoption-draft"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        orphan["draft"]["spec"]["labels"],
        json!({"team":"platform"})
    );
    let adopt_url = url(&f, "adopt");
    let (first, second) = tokio::join!(
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            input(&orphan)
        ),
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            input(&orphan)
        )
    );
    assert!(matches!(
        (first.status(), second.status()),
        (StatusCode::OK, StatusCode::CONFLICT) | (StatusCode::CONFLICT, StatusCode::OK)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM swarmservices WHERE platformid=$1")
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(runtime.lock().await.mutations.is_empty());
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}
