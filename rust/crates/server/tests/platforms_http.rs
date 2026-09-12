#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
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
use citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore;
use citadel_adapters::platform_read_store::PostgresPlatformReadStore;
use citadel_adapters::platform_registration::{
    PlatformRegistrationRuntimeRouter, PostgresPlatformRegistrationStore,
};
use citadel_adapters::resource_metadata_store::PostgresResourceMetadataStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_platforms::{
    InventoryProjectionStore, PlatformReadService, PlatformRegistrationService,
    RuntimeContainerSummary, RuntimeImageSummary, RuntimeInventorySnapshot, RuntimeNetworkSummary,
    RuntimePlatformInfo, RuntimeSwarmConfig, RuntimeSwarmInventory, RuntimeSwarmNode,
    RuntimeSwarmSecret, RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use citadel_server::platforms_http::{self, PlatformsHttpState};
use citadel_server::realtime::{IdentityRealtimeReader, RealtimeReadError, RealtimeReadPort};
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;
use uuid::Uuid;

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[path = "platforms_http/swarm_overview.rs"]
mod swarm_overview;
#[path = "platforms_http/swarm_inventory.rs"]
mod swarm_inventory;
#[path = "platforms_http/service_adoption.rs"]
mod service_adoption;
#[path = "platforms_http/task_runtime.rs"]
mod task_runtime;

#[path = "platforms_http/adoption.rs"]
mod adoption;
#[path = "platforms_http/container_inspection.rs"]
mod container_inspection;
#[path = "platforms_http/container_mutations.rs"]
mod container_mutations;
#[path = "platforms_http/edge.rs"]
mod edge;
#[path = "platforms_http/get_container.rs"]
mod get_container;
#[path = "platforms_http/images.rs"]
mod images;
#[path="platforms_http/platform_image_management.rs"]
mod platform_image_management;
#[path="platforms_http/registry_browsing.rs"]
mod registry_browsing;
#[path = "platforms_http/logs.rs"]
mod logs;
#[path = "platforms_http/lookup.rs"]
mod lookup;
#[path = "platforms_http/node_agent_lifecycle.rs"]
mod node_agent_lifecycle;
#[path = "platforms_http/node_agent_setup.rs"]
mod node_agent_setup;
#[path = "platforms_http/node_coverage.rs"]
mod node_coverage;
#[path = "platforms_http/node_resources.rs"]
mod node_resources;
#[path = "platforms_http/realtime_groups.rs"]
mod realtime_groups;
#[path = "platforms_http/search.rs"]
mod search;
#[path = "platforms_http/statistics.rs"]
mod statistics;
#[path = "platforms_http/volume_content.rs"]
mod volume_content;

struct Fixture {
    app: Router,
    pool: PgPool,
    platform_id: Uuid,
    actor_id: Uuid,
    tag_id: Uuid,
    administrator: ActorPrincipal,
    realtime: IdentityRealtimeReader,
    docker_server: tokio::task::JoinHandle<()>,
    docker_socket: PathBuf,
    lookup_state: citadel_server::lookup_http::LookupHttpState,
    _guard: OwnedMutexGuard<()>,
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn read_routes_enforce_authorization_and_return_persisted_inventory() {
    let fixture = fixture().await;

    assert_eq!(
        send(&fixture, "/api/v1/platforms", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let denied = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: "denied".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    assert_eq!(
        send(
            &fixture,
            &format!("/api/v1/platforms/{}", fixture.platform_id),
            Some(denied.clone()),
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(matches!(
        fixture
            .realtime
            .authorize_platform(&denied, fixture.platform_id)
            .await,
        Err(RealtimeReadError::Authorization)
    ));

    let reader = ActorPrincipal {
        subject_id: fixture.actor_id,
        actor_id: ActorId::new(fixture.actor_id),
        name: "reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    let authorized = json_body(
        send(
            &fixture,
            &format!("/api/v1/platforms?tags={}", fixture.tag_id),
            Some(reader.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(authorized["platforms"].as_array().unwrap().len(), 1);
    assert_eq!(authorized["platforms"][0]["capabilities"]["canRead"], true);
    assert_eq!(
        authorized["platforms"][0]["capabilities"]["canWrite"],
        false
    );
    assert_eq!(
        send(
            &fixture,
            &format!("/api/v1/platforms/{}", fixture.platform_id),
            Some(reader.clone()),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        fixture
            .realtime
            .authorize_platform(&reader, fixture.platform_id)
            .await
            .unwrap()
            .id,
        fixture.platform_id
    );

    let networks = json_body(
        send(
            &fixture,
            &format!("/api/v1/networks/{}", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(networks["networks"][0]["name"], "frontend");
    let network = json_body(
        send(
            &fixture,
            &format!("/api/v1/networks/{}/network-1", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(network["containers"]["container-1"]["Name"], "web");
    assert_eq!(network["peers"][0]["Name"], "worker-1");
    assert_eq!(
        send(
            &fixture,
            &format!(
                "/api/v1/networks/{}/network-1?dockerNodeId=node-1",
                fixture.platform_id
            ),
            Some(fixture.administrator.clone()),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let volumes = json_body(
        send(
            &fixture,
            &format!("/api/v1/volumes/{}", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(volumes["volumes"][0]["name"], "data");
    assert_eq!(volumes["volumes"][0]["usageData"]["size"], 1024);
    assert_eq!(volumes["volumes"][0]["usageData"]["refCount"], 1);
    let volume = json_body(
        send(
            &fixture,
            &format!("/api/v1/volumes/{}/data", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(volume["inUse"], true);
    assert_eq!(volume["usageData"]["size"], 1024);

    let platforms = json_body(
        send(
            &fixture,
            "/api/v1/platforms",
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    let platform = platforms["platforms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|platform| platform["id"] == fixture.platform_id.to_string())
        .expect("the fixture Platform must be returned");
    assert_eq!(platform["capabilities"]["canRead"], true);

    let containers = json_body(
        send(
            &fixture,
            &format!("/api/v1/platforms/{}/containers", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(containers["containers"][0]["name"], "web");
    let expected_ports = json!({
        "80/tcp": [
            {"hostIP": "0.0.0.0", "hostPort": "8080"},
            {"hostIP": "::", "hostPort": "8080"}
        ],
        "443/tcp": []
    });
    assert_eq!(containers["containers"][0]["ports"], expected_ports);
    let container_id = containers["containers"][0]["id"].as_str().unwrap();
    let container = json_body(
        send(
            &fixture,
            &format!("/api/v1/containers/{container_id}"),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(container["ports"], expected_ports);
    let snapshot_containers = fixture
        .realtime
        .list_containers(fixture.platform_id)
        .await
        .unwrap();
    assert_eq!(
        snapshot_containers[0].ports, expected_ports,
        "WebSocket snapshots use the same public shape"
    );
    assert_eq!(
        send(
            &fixture,
            &format!("/api/v1/images/{}", fixture.platform_id),
            Some(fixture.administrator.clone()),
        )
        .await
        .status(),
        StatusCode::OK
    );

    for (collection, detail, id) in [
        ("nodes", "nodes", "node-1"),
        ("services", "services", "service-1"),
        ("tasks", "tasks", "task-1"),
        ("networks", "networks", "network-1"),
        ("configs", "configs", "config-1"),
        ("secrets", "secrets", "secret-1"),
    ] {
        let base = format!(
            "/api/v1/platforms/{}/swarm/{collection}",
            fixture.platform_id
        );
        let response =
            json_body(send(&fixture, &base, Some(fixture.administrator.clone())).await).await;
        assert_eq!(
            response["items"].as_array().unwrap().len(),
            1,
            "{collection}"
        );
        assert_eq!(
            send(
                &fixture,
                &format!("{base}/{id}"),
                Some(fixture.administrator.clone()),
            )
            .await
            .status(),
            StatusCode::OK,
            "{detail} detail"
        );
    }

    let tasks = json_body(
        send(
            &fixture,
            &format!(
                "/api/v1/platforms/{}/swarm/tasks?limit=1&serviceId=service-1",
                fixture.platform_id
            ),
            Some(fixture.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(tasks["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        send(
            &fixture,
            &format!(
                "/api/v1/platforms/{}/swarm/tasks?limit=0",
                fixture.platform_id
            ),
            Some(fixture.administrator.clone()),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &fixture,
            &format!(
                "/api/v1/platforms/{}/swarm/services/missing",
                fixture.platform_id
            ),
            Some(fixture.administrator.clone()),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let updated_platform = json_body(
        send_json(
            &fixture,
            Method::PATCH,
            &format!("/api/v1/platforms/{}/_metadata", fixture.platform_id),
            fixture.administrator.clone(),
            json!({"description":"Swarm manager"}),
        )
        .await,
    )
    .await;
    assert_eq!(updated_platform["description"], "Swarm manager");
    let persisted_description: Option<String> =
        sqlx::query_scalar("SELECT description FROM platforms WHERE id=$1")
            .bind(fixture.platform_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(persisted_description.as_deref(), Some("Swarm manager"));

    let created_network = json_body(
        send_json(
            &fixture,
            Method::POST,
            "/api/v1/networks",
            fixture.administrator.clone(),
            json!({
                "platformId":fixture.platform_id,
                "name":"backend",
                "driver":"overlay",
                "scope":"swarm",
                "attachable":true
            }),
        )
        .await,
    )
    .await;
    assert_eq!(created_network["id"], "network-created");
    // NetworkEndpointTests parity: the default UI has empty IPv4/IPv6 IPAM rows.
    let default_network = json_body(send_json(
        &fixture, Method::POST, "/api/v1/networks", fixture.administrator.clone(),
        json!({"platformId":fixture.platform_id,"name":"fscsd","driver":"bridge","scope":"local",
            "enableIPv4":true,"enableIPv6":false,"internal":false,"attachable":false,"ingress":false,
            "labels":{},"options":{},"ipam":{"driver":"default","config":[{},{}]},"configOnly":false}),
    ).await).await;
    assert_eq!(default_network["id"], "network-created");
    let created_volume = json_body(
        send_json(
            &fixture,
            Method::POST,
            "/api/v1/volumes",
            fixture.administrator.clone(),
            json!({
                "platformId":fixture.platform_id,
                "name":"cache",
                "driver":"local"
            }),
        )
        .await,
    )
    .await;
    assert_eq!(created_volume["name"], "cache");

    fixture.pool.close().await;
    fixture.docker_server.await.unwrap();
    std::fs::remove_file(&fixture.docker_socket).unwrap();
}

async fn fixture() -> Fixture {
    fixture_for_cluster(String::new()).await
}

async fn fixture_for_cluster(cluster: String) -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE4_DATABASE_URL")
        .expect("CITADEL_PHASE4_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let platform_id = Uuid::now_v7();
    let cluster = if cluster.is_empty() {
        format!("cluster-{platform_id}")
    } else {
        cluster
    };
    let actor_id = Uuid::now_v7();
    let tag_id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms (id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,$2,'Local',0,0,0,$3,0,'{\"$type\":\"DockerSwarm\"}','Online',0)")
        .bind(platform_id)
        .bind(format!("unix:///phase4-http/{platform_id}.sock"))
        .bind(format!("phase4-http-{platform_id}"))
        .execute(&pool)
        .await
        .unwrap();
    let mut initial = snapshot(platform_id);
    initial.info.swarm.as_mut().unwrap().cluster_id = Some(cluster.clone());
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&initial)
        .await
        .unwrap();
    seed_reader_access(&pool, platform_id, actor_id, tag_id).await;

    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[7_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let administrator_actor_id = Uuid::now_v7();
    let administrator_user_id = Uuid::now_v7();
    let administrator_name = format!("phase4-admin-{}", administrator_user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(administrator_actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(administrator_user_id)
        .bind(administrator_actor_id)
        .bind(Utc::now())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{administrator_name}@example.test"))
        .bind(&administrator_name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(administrator_actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let administrator = ActorPrincipal {
        subject_id: administrator_user_id,
        actor_id: ActorId::new(administrator_actor_id),
        name: administrator_name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    };
    let (docker, docker_server, docker_socket) = docker_fixture_for_cluster(10, cluster).await;
    let platforms = Arc::new(PlatformReadService::new(Arc::new(
        PostgresPlatformReadStore::new(pool.clone()),
    )));
    let realtime = IdentityRealtimeReader::new(identity.clone(), platforms.clone());
    let registrations = Arc::new(PlatformRegistrationService::new(
        Arc::new(PostgresPlatformRegistrationStore::new(pool.clone())),
        Arc::new(PlatformRegistrationRuntimeRouter::new(docker.clone(), None)),
    ));
    let edge = citadel_adapters::edge::EdgeRegistry::default();
    let platform_state = PlatformsHttpState {
        volume_content: Arc::new(citadel_adapters::volume_content::VolumeContentAdapter::new(
            pool.clone(),
            docker.clone(),
            None,
            edge.clone(),
            "citadel-agent:test".into(),
        )),
        containers: Arc::new(
            citadel_adapters::container_mutations::ContainerRuntimeRouter::new(
                pool.clone(),
                docker.clone(),
                None,
                edge.clone(),
            )
            .into_service(),
        ),
        identity,
        platforms,
        registrations,
        pool: pool.clone(),
        resource_metadata: Arc::new(PostgresResourceMetadataStore::new(pool.clone())),
        docker,
        agent: None,
        edge,
        realtime: None,
        stats_sample_max_age: StdDuration::from_secs(30),
    };
    let lookup_state = citadel_server::lookup_http::LookupHttpState {
        store: Arc::new(citadel_adapters::lookup_store::PostgresLookupStore::new(
            pool.clone(),
        )),
        entitlements: Arc::new(StaticEntitlementService::new(true)),
        platforms: platform_state.clone(),
    };
    let app = platforms_http::router(platform_state)
        .merge(citadel_server::lookup_http::router(lookup_state.clone()));
    Fixture {
        app,
        pool,
        platform_id,
        actor_id,
        tag_id,
        administrator,
        realtime,
        docker_server,
        docker_socket,
        lookup_state,
        _guard: guard,
    }
}

async fn docker_fixture_for_cluster(
    request_limit: usize,
    cluster: String,
) -> (DockerClient, tokio::task::JoinHandle<()>, PathBuf) {
    let socket = std::env::temp_dir().join(format!("citadel-phase4-http-{}.sock", Uuid::now_v7()));
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..request_limit {
            let (mut connection, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = connection.read(&mut chunk).await.unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request_line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap()
                .to_owned();
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap();
            let path = parts.next().unwrap().split('?').next().unwrap();
            let body = match (method, path) {
                ("GET", "/version") => {
                    r#"{"Version":"28.0.0","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
                }
                ("GET", "/v1.49/info") => {
                    r#"{"ID":"daemon-test","NCPU":1,"MemTotal":1048576,"OSType":"linux","Architecture":"x86_64","Swarm":{"NodeID":"node-1","LocalNodeState":"active","ControlAvailable":true,"Nodes":1,"Managers":1,"Cluster":{"ID":"cluster-test"}}}"#
                }
                (
                    "GET",
                    "/v1.49/containers/json"
                    | "/v1.49/images/json"
                    | "/v1.49/services"
                    | "/v1.49/tasks"
                    | "/v1.49/configs"
                    | "/v1.49/secrets",
                ) => "[]",
                ("GET", "/v1.49/nodes") => {
                    r#"[{"ID":"node-1","Spec":{"Role":"manager","Availability":"active","Labels":{}},"Description":{"Hostname":"manager","Platform":{"OS":"linux","Architecture":"x86_64"}},"Status":{"State":"ready"},"ManagerStatus":{"Leader":true,"Reachability":"reachable"}}]"#
                }
                ("GET", "/v1.49/tasks/task-1") => {
                    r#"{"ID":"task-1","NodeID":"node-1","ServiceID":"service-1","Status":{"State":"running","ContainerStatus":{"ContainerID":"container-1"}},"DesiredState":"running"}"#
                }
                ("GET", "/v1.49/services/service-1") => {
                    r#"{"Spec":{"TaskTemplate":{"ContainerSpec":{"TTY":true}}}}"#
                }
                ("GET", "/v1.49/services/service-1/logs") => {
                    "2026-09-06T12:00:00Z task.name=web.1 héllo\n"
                }
                ("GET", "/v1.49/networks") => {
                    r#"[{"Name":"frontend","Id":"network-1","Created":"2026-01-01T00:00:00Z","Scope":"swarm","Driver":"overlay","EnableIPv4":true,"Containers":{"container-1":{}},"Labels":{},"Options":{}}]"#
                }
                ("GET", "/v1.49/networks/network-1") => {
                    r#"{"Name":"frontend","Id":"network-1","Created":"2026-01-01T00:00:00Z","Scope":"swarm","Driver":"overlay","EnableIPv4":true,"Containers":{"container-1":{"Name":"web"}},"Peers":[{"Name":"worker-1","IP":"10.0.0.2"}],"Labels":{},"Options":{}}"#
                }
                ("GET", "/v1.49/volumes") => {
                    r#"{"Volumes":[{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{}}],"Warnings":[]}"#
                }
                ("GET", "/v1.49/volumes/data") => {
                    r#"{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{}}"#
                }
                ("GET", "/v1.49/system/df") => {
                    r#"{"Volumes":[{"Name":"data","UsageData":{"RefCount":1,"Size":1024}}]}"#
                }
                ("POST", "/v1.49/networks/create") => r#"{"Id":"network-created","Warning":""}"#,
                ("POST", "/v1.49/volumes/create") => {
                    r#"{"Name":"cache","Driver":"local","Mountpoint":"/cache","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{},"UsageData":{"RefCount":0,"Size":0}}"#
                }
                unexpected => panic!("unexpected Docker fixture request {unexpected:?}"),
            };
            let body = body.replace("cluster-test", &cluster);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            connection.write_all(response.as_bytes()).await.unwrap();
            connection.shutdown().await.unwrap();
        }
    });
    let client = DockerClient::new(&socket, StdDuration::from_secs(1)).unwrap();
    (client, server, socket)
}

async fn seed_reader_access(pool: &PgPool, platform_id: Uuid, actor_id: Uuid, tag_id: Uuid) {
    let marker = tag_id.simple().to_string();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourceaccesses (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1,$2,1,$3,0,0)")
        .bind(Uuid::now_v7())
        .bind(actor_id)
        .bind(platform_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags (id,color,createdbyactorid,name,normalizedname) VALUES ($1,'blue',$2,$3,$4)")
        .bind(tag_id)
        .bind(actor_id)
        .bind(format!("phase4-http-{marker}"))
        .bind(format!("PHASE4-HTTP-{marker}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourcetags (resourcetype,resourceid,tagid,createdbyactorid) VALUES ('Platform',$1,$2,$3)")
        .bind(platform_id)
        .bind(tag_id)
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
}

fn snapshot(platform_id: Uuid) -> RuntimeInventorySnapshot {
    let observed_at = Utc::now();
    RuntimeInventorySnapshot {
        platform_id,
        info: RuntimePlatformInfo {
            daemon_id: "daemon-test".into(),
            server_version: "28.0.0".into(),
            operating_system: "Linux".into(),
            os_type: "linux".into(),
            architecture: "x86_64".into(),
            cpu_count: 2,
            memory_total: 2048,
            container_count: 1,
            containers_running: 1,
            containers_paused: 0,
            containers_stopped: 0,
            api_version: "1.49".into(),
            minimum_api_version: "1.24".into(),
            agent_version: None,
            swarm: Some(citadel_platforms::RuntimeSwarmInfo {
                node_id: "manager-node".into(),
                cluster_id: Some(format!("cluster-{platform_id}")),
                local_node_state: "active".into(),
                control_available: true,
                ..Default::default()
            }),
        },
        containers: vec![RuntimeContainerSummary {
            id: "container-1".into(),
            name: "web".into(),
            image: "nginx".into(),
            image_id: "sha256:image-1".into(),
            created: 1,
            state: "running".into(),
            status: "Up".into(),
            labels: BTreeMap::new(),
            // Raw rows from older Local projections must work without a reset.
            ports: json!([
                {"PrivatePort": 80, "PublicPort": 8080, "Type": "tcp", "IP": "0.0.0.0"},
                {"PrivatePort": 80, "PublicPort": 8080, "Type": "tcp", "IP": "::"},
                {"PrivatePort": 443, "Type": "tcp", "IP": ""}
            ]),
            stack: None,
            is_system: false,
            system_role: None,
            has_citadel_ownership_labels: false,
            is_swarm_task: false,
        }],
        images: vec![RuntimeImageSummary {
            id: "sha256:image-1".into(),
            repo_tags: vec!["nginx:latest".into()],
            repo_digests: vec![],
            created: 1,
            size: 100,
            containers: 1,
        }],
        networks: vec![RuntimeNetworkSummary {
            id: "network-1".into(),
            name: "frontend".into(),
            driver: "overlay".into(),
            scope: "swarm".into(),
            created: observed_at.to_rfc3339(),
            ..Default::default()
        }],
        volumes: vec![RuntimeVolumeSummary {
            name: "data".into(),
            driver: "local".into(),
            scope: "local".into(),
            ..Default::default()
        }],
        swarm: Some(RuntimeSwarmInventory {
            nodes: vec![RuntimeSwarmNode {
                id: "node-1".into(),
                version_index: 1,
                hostname: "manager".into(),
                role: "manager".into(),
                is_leader: true,
                reachability: "reachable".into(),
                status: "ready".into(),
                availability: "active".into(),
                engine_version: "28".into(),
                operating_system: "linux".into(),
                architecture: "x86_64".into(),
                address: "10.0.0.1".into(),
                ..Default::default()
            }],
            services: vec![
                RuntimeSwarmService {
                    id: "service-1".into(),
                    version_index: 1,
                    name: "web".into(),
                    mode: "Replicated".into(),
                    image: "nginx".into(),
                    running_task_count: 1,
                    desired_task_count: 1,
                    update_state: "completed".into(),
                    network_ids: vec!["network-1".into()],
                    secret_ids: vec!["secret-1".into()],
                    config_ids: vec!["config-1".into()],
                    ..Default::default()
                }
                .normalize_ownership(),
            ],
            tasks: vec![RuntimeSwarmTask {
                id: "task-1".into(),
                version_index: 1,
                name: "web.1".into(),
                service_id: "service-1".into(),
                node_id: "node-1".into(),
                desired_state: "running".into(),
                state: "running".into(),
                image: "nginx".into(),
                ..Default::default()
            }],
            configs: vec![RuntimeSwarmConfig {
                id: "config-1".into(),
                version_index: 1,
                name: "config".into(),
                ..Default::default()
            }],
            secrets: vec![RuntimeSwarmSecret {
                id: "secret-1".into(),
                version_index: 1,
                name: "secret".into(),
                ..Default::default()
            }],
        }),
        observed_at,
    }
}

async fn send(
    fixture: &Fixture,
    uri: &str,
    principal: Option<ActorPrincipal>,
) -> axum::response::Response {
    let mut request = Request::builder().method(Method::GET).uri(uri);
    if let Some(principal) = principal {
        request.extensions_mut().unwrap().insert(principal);
    }
    fixture
        .app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn send_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    principal: ActorPrincipal,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    request.extensions_mut().insert(principal);
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
