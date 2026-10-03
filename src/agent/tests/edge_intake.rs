//! Opt-in handshake and command interoperability with Core's real intake and an isolated test database.
use axum::response::IntoResponse;
use citadel_adapters::{
    connectors::edge::{EdgeIntake, EdgeRegistry, EdgeSession, EdgeTarget},
    persistence::postgres::platforms::edge::store::PostgresEdgeStore,
};
use citadel_agent::{app::Agent, config::AgentConfig};
use citadel_contracts::citadel::edge::v1::{
    EdgeCommandKind, edge_agent_service_server::EdgeAgentServiceServer,
};
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest,
    platforms::v1::{CheckHealthResponse, DaemonEventResponse},
};
use prost::Message;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
struct StreamGuard(Arc<AtomicUsize>);
impl Drop for StreamGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
async fn streams_closed(streams: &AtomicUsize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while streams.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Edge cancellation must close Docker streams");
}

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Fixture {
    directory: PathBuf,
    stop: CancellationToken,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.cancel();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
async fn session(
    registry: &EdgeRegistry,
    target: &EdgeTarget,
    previous: Option<Uuid>,
) -> Arc<EdgeSession> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(session) = registry.get(target)
                && Some(session.id) != previous
            {
                return session;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Agent must authenticate to Core")
}

#[tokio::test]
#[ignore = "requires an isolated CITADEL_AGENT_TEST_DATABASE_URL; creates Platform and Build Pool fixtures"]
async fn rust_agent_enrolls_reconnects_and_restarts_against_core_intake() {
    let url = std::env::var("CITADEL_AGENT_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresEdgeStore::new(pool.clone());
    let registry = EdgeRegistry::default();
    let fixture = Fixture {
        directory: std::env::temp_dir().join(format!("citadel-agent-intake-{}", Uuid::now_v7())),
        stop: CancellationToken::new(),
    };
    std::fs::create_dir(&fixture.directory).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let core_url = format!("http://{}", listener.local_addr().unwrap());
    let stop = fixture.stop.clone();
    let intake = EdgeIntake::new(store.clone(), registry.clone());
    let core = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EdgeAgentServiceServer::new(intake))
            .serve_with_incoming_shutdown(
                futures_util::stream::unfold(listener, |listener| async {
                    Some((listener.accept().await.map(|v| v.0), listener))
                }),
                stop.cancelled_owned(),
            )
            .await
            .unwrap();
    });
    for build in [false, true] {
        let id = Uuid::now_v7();
        let target = if build {
            EdgeTarget::build_pool(id)
        } else {
            EdgeTarget::platform(id)
        };
        if build {
            sqlx::query("INSERT INTO buildagentpools(id,name,normalizedname,createdbyactorid,provider,providerspec) VALUES($1,$2,$2,$3,'SelfManagedVm','{\"$type\":\"SelfManagedVm\",\"ConnectionMode\":\"EdgeAgent\"}')")
                .bind(id).bind(id.to_string()).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
        } else {
            sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'EdgeAgent',0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Offline',0)")
                .bind(id).execute(&pool).await.unwrap();
        }
        let (_, token, _) = store
            .create_enrollment(&target, citadel_identity::SYSTEM_ACTOR_ID)
            .await
            .unwrap();
        let daemon = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker_host = format!("http://{}", daemon.local_addr().unwrap());
        let streams = Arc::new(AtomicUsize::new(0));
        let router = axum::Router::new().fallback({
            let streams = streams.clone();
            move |uri:axum::http::Uri| {
                let streams = streams.clone();
                async move {
                    match uri.path() {
                        "/version" => axum::Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41","Version":"29"})).into_response(),
                        "/_ping" => "OK".into_response(),
                        "/v1.49/info" => axum::Json(serde_json::json!({"ID":format!("fixture-{id}"),"Name":"fixture-host","ServerVersion":"29"})).into_response(),
                        "/v1.49/events" => {
                            streams.fetch_add(1, Ordering::SeqCst);
                            let guard = StreamGuard(streams);
                            axum::body::Body::from_stream(async_stream::stream! {
                                let _guard = guard;
                                yield Ok::<_,std::io::Error>(bytes::Bytes::from_static(b"{\"Type\":\"service\",\"Action\":\"create\",\"Actor\":{\"ID\":\"fixture-service\",\"Attributes\":{}},\"scope\":\"swarm\"}\n"));
                                std::future::pending::<()>().await;
                            }).into_response()
                        }
                        _ => (axum::http::StatusCode::NOT_FOUND, axum::Json(serde_json::json!({"message":"Fixture resource not found"}))).into_response(),
                    }
                }
            }
        });
        let stop = fixture.stop.clone();
        tokio::spawn(async move {
            axum::serve(daemon, router)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
                .unwrap();
        });
        let key_path = fixture.directory.join(format!("{id}.key"));
        let identity_path = fixture.directory.join(format!("{id}.json"));
        let make_config = |enroll: bool| {
            let mut config = AgentConfig::from_lookup(|name| match name {
                "CITADEL_AGENT_MODE" => Some("edge".into()),
                "CITADEL_CORE_URL" => Some(core_url.clone()),
                "CITADEL_EDGE_AGENT_PROFILE" => Some(
                    if build {
                        "edge-build-agent"
                    } else {
                        "edge-agent"
                    }
                    .into(),
                ),
                "CITADEL_EDGE_ENROLLMENT_TOKEN" => enroll.then(|| token.to_string()),
                "CITADEL_EDGE_AGENT_KEY_PATH" => Some(key_path.display().to_string()),
                "CITADEL_EDGE_IDENTITY_PATH" => Some(identity_path.display().to_string()),
                "DOCKER_HOST" => Some(docker_host.clone()),
                _ => None,
            })
            .unwrap();
            config.port = 0;
            config
        };
        let agent = Agent::bind(make_config(true)).await.unwrap();
        let stop = fixture.stop.child_token();
        let running = tokio::spawn(agent.serve(stop.clone()));
        let first = session(&registry, &target, None).await;
        tokio::time::timeout(Duration::from_secs(3), async {
            while !identity_path.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let key = std::fs::read(&key_path).unwrap();
        let state: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&identity_path).unwrap()).unwrap();
        assert_eq!(state["ResourceId"], target.resource_id.to_string());
        assert_eq!(state["PlatformId"], target.platform_id.to_string());
        registry.disconnect(&target);
        let second = session(&registry, &target, Some(first.id)).await;
        assert_eq!(first.agent_id, second.agent_id);
        let mut command = second
            .command(
                EdgeCommandKind::PlatformCheckHealth,
                vec![],
                Duration::from_secs(2),
                false,
            )
            .unwrap();
        let payload = tokio::time::timeout(
            Duration::from_secs(3),
            command.next(&CancellationToken::new()),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        assert!(
            CheckHealthResponse::decode(payload.as_slice())
                .unwrap()
                .healthy
        );
        assert!(
            command
                .next(&CancellationToken::new())
                .await
                .unwrap()
                .is_none()
        );
        let mut missing = second
            .command(
                EdgeCommandKind::ContainerInspect,
                InspectContainerRequest {
                    container_id: "missing".into(),
                }
                .encode_to_vec(),
                Duration::from_secs(2),
                false,
            )
            .unwrap();
        assert!(missing.next(&CancellationToken::new()).await.is_err());
        let mut events = second
            .command(
                EdgeCommandKind::PlatformDaemonEventsStream,
                vec![],
                Duration::from_secs(30),
                true,
            )
            .unwrap();
        let payload = tokio::time::timeout(
            Duration::from_secs(3),
            events.next(&CancellationToken::new()),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        let event = DaemonEventResponse::decode(payload.as_slice()).unwrap();
        assert!(event.kind.is_some());
        assert_eq!(event.scope, 2);
        drop(events); // Core sends CancelCommand when its consumer leaves.
        streams_closed(&streams).await;
        let mut events = second
            .command(
                EdgeCommandKind::PlatformDaemonEventsStream,
                vec![],
                Duration::from_secs(30),
                true,
            )
            .unwrap();
        assert!(
            events
                .next(&CancellationToken::new())
                .await
                .unwrap()
                .is_some()
        );
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        streams_closed(&streams).await;
        assert!(events.next(&CancellationToken::new()).await.is_err());
        let agent = Agent::bind(make_config(false)).await.unwrap();
        let stop = fixture.stop.child_token();
        let running = tokio::spawn(agent.serve(stop.clone()));
        let third = session(&registry, &target, Some(second.id)).await;
        assert_eq!(third.agent_id, first.agent_id);
        assert_eq!(std::fs::read(&key_path).unwrap(), key);
        store.revoke(&target).await.unwrap();
        registry.disconnect(&target);
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(registry.get(&target).is_err());
        assert_eq!(std::fs::read(&key_path).unwrap(), key);
        assert!(
            identity_path.exists(),
            "rejection without an enrollment credential must preserve state"
        );
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if build {
            // A Core database outage is retryable, not an enrollment rejection.
            pool.close().await;
            let agent = Agent::bind(make_config(true)).await.unwrap();
            let stop = fixture.stop.child_token();
            let running = tokio::spawn(agent.serve(stop.clone()));
            tokio::time::sleep(Duration::from_secs(2)).await;
            assert!(!running.is_finished());
            assert_eq!(std::fs::read(&key_path).unwrap(), key);
            assert!(identity_path.exists());
            stop.cancel();
            tokio::time::timeout(Duration::from_secs(3), running)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
    }
    fixture.stop.cancel();
    tokio::time::timeout(Duration::from_secs(3), core)
        .await
        .unwrap()
        .unwrap();
    pool.close().await;
}
