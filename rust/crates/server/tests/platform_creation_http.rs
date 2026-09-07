#![cfg(unix)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use chrono::{Duration, Utc};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::docker::DockerClient;
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::platform_read_store::PostgresPlatformReadStore;
use citadel_adapters::platform_registration::PostgresPlatformRegistrationStore;
use citadel_adapters::resource_metadata_store::PostgresResourceMetadataStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_platforms::{
    CreatePlatformInput, PlatformConnectorType, PlatformInventoryPort, PlatformReadService,
    PlatformRegistrationError, PlatformRegistrationRuntime, PlatformRegistrationService,
    PlatformRuntimePort, RuntimeCapabilityError, RuntimeContainerSummary, RuntimeErrorKind,
    RuntimeImageSummary, RuntimeNetworkSummary, RuntimePlatformInfo, RuntimeStatsStream,
    RuntimeSwarmConfig, RuntimeSwarmNode, RuntimeSwarmSecret, RuntimeSwarmService,
    RuntimeSwarmTask, RuntimeVolumeSummary,
};
use citadel_server::metrics::Metrics;
use citadel_server::platforms_http::{self, PlatformsHttpState};
use citadel_server::realtime::RealtimeHub;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "platform_creation_http/deletion.rs"]
mod deletion;

struct StaticInventory {
    calls: AtomicUsize,
    info: InfoOutcome,
    containers: Vec<RuntimeContainerSummary>,
    images: Vec<RuntimeImageSummary>,
    nodes: Vec<RuntimeSwarmNode>,
    services: Vec<RuntimeSwarmService>,
    tasks: Vec<RuntimeSwarmTask>,
}

#[derive(Clone)]
enum InfoOutcome {
    Value(Box<RuntimePlatformInfo>),
    Error(RuntimeErrorKind, String, bool),
}

impl StaticInventory {
    fn standalone(daemon_id: impl Into<String>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            info: InfoOutcome::Value(Box::new(RuntimePlatformInfo {
                daemon_id: daemon_id.into(),
                server_version: "28.0.0".into(),
                operating_system: "Linux".into(),
                os_type: "linux".into(),
                architecture: "x86_64".into(),
                cpu_count: 4,
                memory_total: 8 * 1024 * 1024 * 1024,
                container_count: 1,
                containers_running: 1,
                containers_paused: 0,
                containers_stopped: 0,
                api_version: "1.49".into(),
                minimum_api_version: "1.41".into(),
                agent_version: Some("test-agent".into()),
                swarm: None,
            })),
            containers: vec![container("docker-container-1", "running", false)],
            images: vec![RuntimeImageSummary {
                id: "sha256:image-1".into(),
                repo_tags: vec!["nginx:latest".into()],
                created: 1,
                size: 100,
                containers: 1,
                ..Default::default()
            }],
            nodes: Vec::new(),
            services: Vec::new(),
            tasks: Vec::new(),
        }
    }

    fn swarm(daemon_id: impl Into<String>, cluster_id: impl Into<String>) -> Self {
        let mut inventory = Self::standalone(daemon_id);
        let cluster_id = cluster_id.into();
        let InfoOutcome::Value(info) = &mut inventory.info else {
            unreachable!()
        };
        info.container_count = 2;
        info.containers_stopped = 1;
        info.swarm = Some(citadel_platforms::RuntimeSwarmInfo {
            node_id: "manager-1".into(),
            node_addr: "10.0.0.1".into(),
            local_node_state: "active".into(),
            control_available: true,
            nodes: 1,
            managers: 1,
            cluster_id: Some(cluster_id),
            ..Default::default()
        });
        inventory.containers = vec![
            container("current-task", "running", true),
            container("historical-task", "exited", true),
        ];
        inventory.nodes = vec![RuntimeSwarmNode {
            id: "manager-1".into(),
            hostname: "manager".into(),
            role: "manager".into(),
            is_leader: true,
            status: "ready".into(),
            availability: "active".into(),
            ..Default::default()
        }];
        inventory
    }

    fn error(kind: RuntimeErrorKind, message: impl Into<String>) -> Self {
        let mut inventory = Self::standalone("unused");
        inventory.info = InfoOutcome::Error(kind, message.into(), false);
        inventory
    }
}

impl PlatformRuntimePort for StaticInventory {
    fn get_info<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let outcome = self.info.clone();
        Box::pin(async move {
            match outcome {
                InfoOutcome::Value(info) => Ok(*info),
                InfoOutcome::Error(kind, message, retryable) => {
                    Err(RuntimeCapabilityError::new(kind, message, retryable))
                }
            }
        })
    }

    fn list_containers<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let containers = self.containers.clone();
        Box::pin(async move { Ok(containers) })
    }

    fn stream_stats<'a>(
        &'a self,
        _fetch_interval: StdDuration,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
        unreachable!("registration does not stream statistics")
    }
}

macro_rules! static_list {
    ($name:ident, $type:ty, $field:ident) => {
        fn $name<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<$type>, RuntimeCapabilityError>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let values = self.$field.clone();
            Box::pin(async move { Ok(values) })
        }
    };
}

impl PlatformInventoryPort for StaticInventory {
    static_list!(list_images, RuntimeImageSummary, images);
    static_list!(list_swarm_nodes, RuntimeSwarmNode, nodes);
    static_list!(list_swarm_services, RuntimeSwarmService, services);
    static_list!(list_swarm_tasks, RuntimeSwarmTask, tasks);

    fn list_networks<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn list_volumes<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn list_swarm_configs<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn list_swarm_secrets<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn inspect_network<'a>(
        &'a self,
        _id: &'a str,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        unreachable!("registration does not inspect individual networks")
    }

    fn inspect_volume<'a>(
        &'a self,
        _name: &'a str,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        unreachable!("registration does not inspect individual volumes")
    }
}

struct StaticRegistrationRuntime {
    inventory: StaticInventory,
    selections: Mutex<Vec<(PlatformConnectorType, String)>>,
}

impl PlatformRegistrationRuntime for StaticRegistrationRuntime {
    fn inventory_for<'a>(
        &'a self,
        connector_type: PlatformConnectorType,
        address: &'a str,
    ) -> Result<&'a dyn PlatformInventoryPort, PlatformRegistrationError> {
        self.selections
            .lock()
            .unwrap()
            .push((connector_type, address.to_owned()));
        Ok(&self.inventory)
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn create_platform_enforces_authorization_and_atomically_persists_initial_inventory() {
    let database_url = std::env::var("CITADEL_PHASE4_DATABASE_URL")
        .expect("CITADEL_PHASE4_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[61_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let (administrator, denied) = seed_actors(&pool).await;
    let runtime = Arc::new(StaticRegistrationRuntime {
        inventory: StaticInventory::standalone(format!("daemon-{}", Uuid::now_v7().simple())),
        selections: Mutex::new(Vec::new()),
    });
    let registrations = Arc::new(PlatformRegistrationService::new(
        Arc::new(PostgresPlatformRegistrationStore::new(pool.clone())),
        runtime.clone(),
    ));
    let socket = std::env::temp_dir().join(format!("unused-{}.sock", Uuid::now_v7()));
    let public_key =
        citadel_adapters::agent::AgentRequestSigner::from_bytes(&[71; 32]).public_key_base64();
    let setup = Arc::new(citadel_platforms::agent_setup::AgentSetupView::new(
        public_key.clone(),
        "citadel-agent:test".into(),
        false,
    ));
    let app = platforms_http::router(PlatformsHttpState {
        volume_content: Arc::new(citadel_adapters::volume_content::VolumeContentAdapter::new(
            pool.clone(),
            DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
            None,
            citadel_adapters::edge::EdgeRegistry::default(),
            "citadel-agent:test".into(),
        )),
        containers: Arc::new(
            citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
                pool.clone(),
                DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
                None,
                citadel_adapters::edge::EdgeRegistry::default(),
            )
            .into_service(),
        ),
        identity,
        platforms: Arc::new(PlatformReadService::new(Arc::new(
            PostgresPlatformReadStore::new(pool.clone()),
        ))),
        registrations,
        pool: pool.clone(),
        resource_metadata: Arc::new(PostgresResourceMetadataStore::new(pool.clone())),
        docker: DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
        agent: None,
        edge: citadel_adapters::edge::EdgeRegistry::default(),
        realtime: None,
        stats_sample_max_age: StdDuration::from_secs(30),
    })
    .layer(axum::Extension(setup));

    for (principal, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(denied.clone()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(agent_setup_request(&app, principal).await.status(), status);
    }

    let input = json!({
        "name": format!("local-{}", Uuid::now_v7().simple()),
        "address": null,
        "description": "Local Docker",
        "type": "Docker",
        "connectorType": "Local",
        "pruneHistoricalSwarmTaskContainers": true,
        "tagIds": []
    });
    assert_eq!(
        request(&app, None, input.clone()).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&app, Some(denied), input.clone()).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(runtime.inventory.calls.load(Ordering::Relaxed), 0);

    let invalid = json!({"name":"bad name","address":null,"type":"Docker","connectorType":"Local"});
    assert_eq!(
        request(&app, Some(administrator.clone()), invalid)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(runtime.inventory.calls.load(Ordering::Relaxed), 0);

    let response = request(&app, Some(administrator.clone()), input.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    let platform_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["address"], "http://localhost.docker");
    assert_eq!(body["imageCount"], 1);
    assert_eq!(runtime.inventory.calls.load(Ordering::Relaxed), 5);

    let persisted: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM platforms WHERE id=$1), (SELECT COUNT(*) FROM images WHERE platformid=$1), (SELECT COUNT(*) FROM containers WHERE platformid=$1)",
    )
    .bind(platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted, (1, 1, 1));
    // The unchanged Platform form requests this endpoint after creation. It
    // must not touch Docker, rotate keys or expose private signing material.
    for _ in 0..2 {
        let setup = agent_setup_request(&app, Some(administrator.clone())).await;
        assert_eq!(setup.status(), StatusCode::OK);
        assert!(
            setup.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        let data: Value =
            serde_json::from_slice(&to_bytes(setup.into_body(), 64 * 1024).await.unwrap()).unwrap();
        assert_eq!(data["hubPublicKey"], public_key);
        assert_eq!(data["environment"]["HUB_PUBLIC_KEY"], public_key);
        assert_eq!(data["agentImage"], "citadel-agent:test");
        assert_eq!(data["requiresTls"], false);
        assert!(
            data["dockerRunCommand"]
                .as_str()
                .unwrap()
                .contains("-p 9000:9000")
        );
        assert!(!data.to_string().contains("PRIVATE_KEY="));
    }
    assert_eq!(runtime.inventory.calls.load(Ordering::Relaxed), 5);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformCreated' AND status='Success'",
    )
    .bind(platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);

    let duplicate = json!({
        "name": input["name"].clone(),
        "address": null,
        "type": "Docker"
    });
    assert_eq!(
        request(&app, Some(administrator), duplicate).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        runtime.inventory.calls.load(Ordering::Relaxed),
        5,
        "known name/address conflicts must not contact Docker"
    );

    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM containers WHERE platformid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM images WHERE platformid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

async fn agent_setup_request(
    app: &Router,
    actor: Option<ActorPrincipal>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .uri("/api/v1/platforms/agent/setup")
        .body(Body::empty())
        .unwrap();
    if let Some(actor) = actor {
        request.extensions_mut().insert(actor);
    }
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn agent_platform_creation_persists_tags_transport_and_realtime_change() {
    let suffix = Uuid::now_v7().simple().to_string();
    let name = format!("agent-{suffix}");
    let address = format!("https://agent-{suffix}.example.test:5001");
    let harness = harness(StaticInventory::standalone(format!("daemon-{suffix}"))).await;
    let tag_id = seed_tag(&harness.pool, &suffix).await;
    let mut events = harness.realtime.subscribe();

    let response = request(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({
            "name": name,
            "address": format!("{address}/"),
            "description": "Remote Docker agent",
            "type": "Docker",
            "connectorType": "Agent",
            "tagIds": [tag_id]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let response_body = response_json(response).await;
    let platform_id = Uuid::parse_str(response_body["id"].as_str().unwrap()).unwrap();
    let persisted: (String, String, i64, i64, i64) = sqlx::query_as(
        r#"SELECT address, connectortype,
                  (SELECT COUNT(*) FROM images WHERE platformid=platforms.id),
                  (SELECT COUNT(*) FROM containers WHERE platformid=platforms.id),
                  (SELECT COUNT(*) FROM resourcetags WHERE resourcetype='Platform' AND resourceid=platforms.id)
           FROM platforms WHERE id=$1"#,
    )
    .bind(platform_id)
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(persisted, (address.clone(), "Agent".into(), 1, 1, 1));
    assert_eq!(
        harness.runtime.selections.lock().unwrap().as_slice(),
        [(PlatformConnectorType::Agent, address)]
    );
    assert!(
        events.try_recv().is_ok(),
        "creation must notify realtime readers"
    );
    cleanup_platform(&harness.pool, platform_id).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn swarm_manager_creation_persists_cluster_inventory_and_prunes_historical_tasks() {
    let suffix = Uuid::now_v7().simple().to_string();
    let cluster_id = format!("cluster-{suffix}");
    let name = format!("swarm-{suffix}");
    let harness = harness(StaticInventory::swarm(
        format!("daemon-{suffix}"),
        cluster_id.clone(),
    ))
    .await;

    let response = request(
        &harness.app,
        Some(harness.administrator.clone()),
        json!({
            "name": name,
            "address": format!("https://manager-{suffix}.example.test:5001"),
            "type": "DockerSwarm",
            "connectorType": "Agent"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let platform_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    let persisted: (String, String, bool, i64, i64) = sqlx::query_as(
        r#"SELECT clusterid, platformdescriptor::jsonb->>'$type', prunehistoricalswarmtaskcontainers,
                  (SELECT COUNT(*) FROM containers WHERE platformid=platforms.id),
                  (SELECT COUNT(*) FROM swarmnodeprojections WHERE platformid=platforms.id)
           FROM platforms WHERE id=$1"#,
    )
    .bind(platform_id)
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(persisted, (cluster_id, "DockerSwarm".into(), true, 1, 1));
    let container_id: String =
        sqlx::query_scalar("SELECT dockercontainerid FROM containers WHERE platformid=$1")
            .bind(platform_id)
            .fetch_one(&harness.pool)
            .await
            .unwrap();
    assert_eq!(container_id, "current-task");
    assert_eq!(harness.runtime.inventory.calls.load(Ordering::Relaxed), 10);
    cleanup_platform(&harness.pool, platform_id).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_creation_preserves_runtime_errors_and_rejects_invalid_daemon_types() {
    let authentication = harness(StaticInventory::error(
        RuntimeErrorKind::Authentication,
        "Agent rejected the Core signature.",
    ))
    .await;
    let response = request(
        &authentication.app,
        Some(authentication.administrator.clone()),
        agent_input("auth-failure", "Docker"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        response_text(response)
            .await
            .contains("Agent rejected the Core signature.")
    );

    let missing_id = harness(StaticInventory::standalone("  ")).await;
    let response = request(
        &missing_id.app,
        Some(missing_id.administrator.clone()),
        agent_input("missing-daemon", "Docker"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(platform_count(&missing_id.pool, "missing-daemon").await, 0);

    let active_swarm = harness(StaticInventory::swarm("daemon-active", "cluster-active")).await;
    let response = request(
        &active_swarm.app,
        Some(active_swarm.administrator.clone()),
        agent_input("wrong-standalone", "Docker"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_text(response)
            .await
            .contains("active Swarm member")
    );
    assert_eq!(
        active_swarm.runtime.inventory.calls.load(Ordering::Relaxed),
        1
    );

    let mut worker_inventory = StaticInventory::swarm("daemon-worker", "cluster-worker");
    let InfoOutcome::Value(worker_info) = &mut worker_inventory.info else {
        unreachable!()
    };
    worker_info.swarm.as_mut().unwrap().control_available = false;
    let worker = harness(worker_inventory).await;
    let response = request(
        &worker.app,
        Some(worker.administrator.clone()),
        agent_input("swarm-worker", "DockerSwarm"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_text(response).await.contains("manager node"));
    assert_eq!(worker.runtime.inventory.calls.load(Ordering::Relaxed), 1);

    let mut missing_cluster_inventory =
        StaticInventory::swarm("daemon-no-cluster", "discarded-cluster");
    let InfoOutcome::Value(missing_cluster_info) = &mut missing_cluster_inventory.info else {
        unreachable!()
    };
    missing_cluster_info.swarm.as_mut().unwrap().cluster_id = None;
    let missing_cluster = harness(missing_cluster_inventory).await;
    let response = request(
        &missing_cluster.app,
        Some(missing_cluster.administrator.clone()),
        agent_input("missing-cluster", "DockerSwarm"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_text(response).await.contains("cluster id"));
    assert_eq!(
        missing_cluster
            .runtime
            .inventory
            .calls
            .load(Ordering::Relaxed),
        1
    );

    let unsupported = harness(StaticInventory::standalone("unused-daemon")).await;
    let response = request(
        &unsupported.app,
        Some(unsupported.administrator),
        agent_input("unsupported-platform", "Kubernetes"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        unsupported.runtime.inventory.calls.load(Ordering::Relaxed),
        0
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_creation_rejects_address_daemon_and_cluster_duplicates() {
    let suffix = Uuid::now_v7().simple().to_string();
    let daemon_id = format!("duplicate-daemon-{suffix}");
    let duplicate_harness = harness(StaticInventory::standalone(daemon_id)).await;
    let address = format!("https://first-{suffix}.example.test:5001");
    let first_name = format!("first-{suffix}");
    let first = request(
        &duplicate_harness.app,
        Some(duplicate_harness.administrator.clone()),
        json!({"name":first_name,"address":address,"type":"Docker","connectorType":"Agent"}),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_id = Uuid::parse_str(response_json(first).await["id"].as_str().unwrap()).unwrap();
    let calls_after_create = duplicate_harness
        .runtime
        .inventory
        .calls
        .load(Ordering::Relaxed);

    let duplicate_address = request(
        &duplicate_harness.app,
        Some(duplicate_harness.administrator.clone()),
        json!({"name":format!("address-{suffix}"),"address":address,"type":"Docker","connectorType":"Agent"}),
    )
    .await;
    assert_eq!(duplicate_address.status(), StatusCode::CONFLICT);
    assert_eq!(
        duplicate_harness
            .runtime
            .inventory
            .calls
            .load(Ordering::Relaxed),
        calls_after_create,
        "known address conflicts must not contact the runtime"
    );

    let duplicate_daemon = request(
        &duplicate_harness.app,
        Some(duplicate_harness.administrator),
        json!({"name":format!("daemon-{suffix}"),"address":format!("https://second-{suffix}.example.test:5001"),"type":"Docker","connectorType":"Agent"}),
    )
    .await;
    assert_eq!(duplicate_daemon.status(), StatusCode::CONFLICT);
    assert!(response_text(duplicate_daemon).await.contains(&first_name));

    let duplicate_daemon_from_local = request(
        &duplicate_harness.app,
        Some(seed_actor(&duplicate_harness.pool, true).await),
        json!({"name":format!("local-daemon-{suffix}"),"type":"Docker","connectorType":"Local"}),
    )
    .await;
    assert_eq!(duplicate_daemon_from_local.status(), StatusCode::CONFLICT);
    assert!(
        response_text(duplicate_daemon_from_local)
            .await
            .contains(&first_name)
    );

    let cluster_id = format!("duplicate-cluster-{suffix}");
    let first_cluster = harness(StaticInventory::swarm(
        format!("cluster-daemon-a-{suffix}"),
        cluster_id.clone(),
    ))
    .await;
    let response = request(
        &first_cluster.app,
        Some(first_cluster.administrator.clone()),
        json!({"name":format!("cluster-a-{suffix}"),"address":format!("https://cluster-a-{suffix}.example.test"),"type":"DockerSwarm","connectorType":"Agent"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cluster_platform_id =
        Uuid::parse_str(response_json(response).await["id"].as_str().unwrap()).unwrap();

    let second_cluster = harness(StaticInventory::swarm(
        format!("cluster-daemon-b-{suffix}"),
        cluster_id,
    ))
    .await;
    let response = request(
        &second_cluster.app,
        Some(second_cluster.administrator),
        json!({"name":format!("cluster-b-{suffix}"),"address":format!("https://cluster-b-{suffix}.example.test"),"type":"DockerSwarm","connectorType":"Agent"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(response_text(response).await.contains("Swarm cluster"));

    cleanup_platform(&duplicate_harness.pool, first_id).await;
    cleanup_platform(&first_cluster.pool, cluster_platform_id).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_creation_rolls_back_invalid_tags_and_serializes_competing_creates() {
    let suffix = Uuid::now_v7().simple().to_string();
    let invalid_tag = harness(StaticInventory::standalone(format!("tag-daemon-{suffix}"))).await;
    let name = format!("invalid-tag-{suffix}");
    let response = request(
        &invalid_tag.app,
        Some(invalid_tag.administrator.clone()),
        json!({
            "name": name,
            "address": format!("https://invalid-tag-{suffix}.example.test"),
            "type": "Docker",
            "connectorType": "Agent",
            "tagIds": [Uuid::now_v7()]
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(platform_count(&invalid_tag.pool, &name).await, 0);

    let rollback_name = format!("rollback-{suffix}");
    let rollback_runtime = Arc::new(StaticRegistrationRuntime {
        inventory: StaticInventory::standalone(format!("rollback-daemon-{suffix}")),
        selections: Mutex::new(Vec::new()),
    });
    let rollback_service = PlatformRegistrationService::new(
        Arc::new(PostgresPlatformRegistrationStore::new(
            invalid_tag.pool.clone(),
        )),
        rollback_runtime,
    );
    let rollback_input: CreatePlatformInput = serde_json::from_value(json!({
        "name": rollback_name,
        "address": format!("https://rollback-{suffix}.example.test"),
        "type": "Docker",
        "connectorType": "Agent"
    }))
    .unwrap();
    let error = rollback_service
        .create(
            ActorId::new(Uuid::now_v7()),
            rollback_input,
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, PlatformRegistrationError::Storage(_)));
    assert_eq!(platform_count(&invalid_tag.pool, &rollback_name).await, 0);

    let competing = harness(StaticInventory::standalone(format!("race-daemon-{suffix}"))).await;
    let input = json!({
        "name": format!("race-{suffix}"),
        "address": format!("https://race-{suffix}.example.test"),
        "type": "Docker",
        "connectorType": "Agent"
    });
    let (left, right) = tokio::join!(
        request(
            &competing.app,
            Some(competing.administrator.clone()),
            input.clone()
        ),
        request(&competing.app, Some(competing.administrator.clone()), input)
    );
    let mut statuses = [left.status(), right.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    let platform_id: Uuid = sqlx::query_scalar("SELECT id FROM platforms WHERE name=$1")
        .bind(format!("race-{suffix}"))
        .fetch_one(&competing.pool)
        .await
        .unwrap();
    cleanup_platform(&competing.pool, platform_id).await;
}

struct TestHarness {
    app: Router,
    pool: sqlx::PgPool,
    administrator: ActorPrincipal,
    runtime: Arc<StaticRegistrationRuntime>,
    realtime: RealtimeHub,
    edge: citadel_adapters::edge::EdgeRegistry,
}

async fn harness(inventory: StaticInventory) -> TestHarness {
    let database_url = std::env::var("CITADEL_PHASE4_DATABASE_URL")
        .expect("CITADEL_PHASE4_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[61_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let administrator = seed_actor(&pool, true).await;
    let runtime = Arc::new(StaticRegistrationRuntime {
        inventory,
        selections: Mutex::new(Vec::new()),
    });
    let registrations = Arc::new(PlatformRegistrationService::new(
        Arc::new(PostgresPlatformRegistrationStore::new(pool.clone())),
        runtime.clone(),
    ));
    let realtime = RealtimeHub::new(16, Arc::new(Metrics::default()));
    let edge = citadel_adapters::edge::EdgeRegistry::default();
    let socket = std::env::temp_dir().join(format!("unused-{}.sock", Uuid::now_v7()));
    let app = platforms_http::router(PlatformsHttpState {
        volume_content: Arc::new(citadel_adapters::volume_content::VolumeContentAdapter::new(
            pool.clone(),
            DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
            None,
            citadel_adapters::edge::EdgeRegistry::default(),
            "citadel-agent:test".into(),
        )),
        containers: Arc::new(
            citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
                pool.clone(),
                DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
                None,
                citadel_adapters::edge::EdgeRegistry::default(),
            )
            .into_service(),
        ),
        identity,
        platforms: Arc::new(PlatformReadService::new(Arc::new(
            PostgresPlatformReadStore::new(pool.clone()),
        ))),
        registrations,
        pool: pool.clone(),
        resource_metadata: Arc::new(PostgresResourceMetadataStore::new(pool.clone())),
        docker: DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap(),
        agent: None,
        realtime: Some(realtime.clone()),
        edge: edge.clone(),
        stats_sample_max_age: StdDuration::from_secs(30),
    });
    TestHarness {
        app,
        pool,
        administrator,
        runtime,
        realtime,
        edge,
    }
}

fn agent_input(name: &str, platform_type: &str) -> Value {
    json!({
        "name": name,
        "address": format!("https://{name}.example.test:5001"),
        "type": platform_type,
        "connectorType": "Agent"
    })
}

fn container(id: &str, state: &str, is_swarm_task: bool) -> RuntimeContainerSummary {
    RuntimeContainerSummary {
        id: id.into(),
        name: id.into(),
        image: "nginx:latest".into(),
        image_id: "sha256:image-1".into(),
        created: 1,
        state: state.into(),
        status: state.into(),
        labels: Default::default(),
        ports: json!([]),
        stack: None,
        is_system: false,
        system_role: None,
        has_citadel_ownership_labels: false,
        is_swarm_task,
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

async fn response_text(response: axum::response::Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

async fn platform_count(pool: &sqlx::PgPool, name: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM platforms WHERE name=$1")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_tag(pool: &sqlx::PgPool, suffix: &str) -> Uuid {
    let id = Uuid::now_v7();
    let name = format!("platform-tag-{suffix}");
    sqlx::query(
        "INSERT INTO tags (id,color,createdbyactorid,name,normalizedname) VALUES ($1,'#112233',$2,$3,$4)",
    )
    .bind(id)
    .bind(SYSTEM_ACTOR_ID)
    .bind(&name)
    .bind(name.to_uppercase())
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn cleanup_platform(pool: &sqlx::PgPool, platform_id: Uuid) {
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Platform' AND resourceid=$1")
        .bind(platform_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(platform_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_actors(pool: &sqlx::PgPool) -> (ActorPrincipal, ActorPrincipal) {
    let administrator = seed_actor(pool, true).await;
    let denied = seed_actor(pool, false).await;
    (administrator, denied)
}

async fn seed_actor(pool: &sqlx::PgPool, administrator: bool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("platform-create-{}", user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id,isenabled,type) VALUES ($1,TRUE,'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id,actorid,createdat,createdbyactorid,email,name) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(user_id)
        .bind(actor_id)
        .bind(Utc::now())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{name}@example.test"))
        .bind(&name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    if administrator {
        sqlx::query("INSERT INTO actorroles (actorid,roleid) VALUES ($1,$2)")
            .bind(actor_id)
            .bind(ADMIN_ROLE_ID)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    transaction.commit().await.unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: administrator
            .then(|| "Admin".to_owned())
            .into_iter()
            .collect(),
    }
}

async fn request(
    app: &Router,
    principal: Option<ActorPrincipal>,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/platforms")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}
