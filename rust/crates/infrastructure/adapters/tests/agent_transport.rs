use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
use citadel_contracts::citadel::platforms::v1::platform_service_server::{
    PlatformService, PlatformServiceServer,
};
use citadel_contracts::citadel::platforms::v1::{
    CheckHealthResponse, DaemonEventResponse, DaemonResourceEventResponse, PlatformStatsRequest,
    PlatformStatsResponse, PruneRequest, PruneResponse, daemon_event_response,
};
use citadel_contracts::citadel::shared_models::v1::{PlatformInfoResponse, PlatformStatMessage};
use citadel_platforms::{PlatformHealthPort, PlatformRuntimePort, RuntimeErrorKind};
use futures_util::{Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

type StatsStream =
    Pin<Box<dyn Stream<Item = Result<PlatformStatsResponse, Status>> + Send + 'static>>;
type EventStream =
    Pin<Box<dyn Stream<Item = Result<DaemonEventResponse, Status>> + Send + 'static>>;

#[derive(Clone, Copy)]
enum HandshakeBehavior {
    FailOnce,
    Delay,
}

#[derive(Clone)]
struct FixtureService {
    behavior: HandshakeBehavior,
    attempts: Arc<AtomicUsize>,
    nonces: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[tonic::async_trait]
impl PlatformService for FixtureService {
    async fn get_platform_info(
        &self,
        request: Request<()>,
    ) -> Result<Response<PlatformInfoResponse>, Status> {
        let nonce = request
            .metadata()
            .get_bin("x-nonce-bin")
            .ok_or_else(|| Status::unauthenticated("missing nonce"))?
            .to_bytes()
            .map_err(|_| Status::unauthenticated("invalid nonce"))?
            .to_vec();
        self.nonces.lock().await.push(nonce);
        let attempt = self.attempts.fetch_add(1, Ordering::Relaxed);
        match self.behavior {
            HandshakeBehavior::FailOnce if attempt == 0 => {
                return Err(Status::unavailable("temporary"));
            }
            HandshakeBehavior::Delay => tokio::time::sleep(Duration::from_millis(100)).await,
            HandshakeBehavior::FailOnce => {}
        }
        Ok(Response::new(platform_info()))
    }

    async fn check_health(
        &self,
        request: Request<()>,
    ) -> Result<Response<CheckHealthResponse>, Status> {
        assert!(request.metadata().get_bin("x-signature-bin").is_some());
        assert!(request.metadata().get_bin("x-nonce-bin").is_some());
        Ok(Response::new(CheckHealthResponse { healthy: true }))
    }

    async fn prune(
        &self,
        request: Request<PruneRequest>,
    ) -> Result<Response<PruneResponse>, Status> {
        assert!(request.metadata().get_bin("x-signature-bin").is_some());
        let attempt = self.attempts.fetch_add(1, Ordering::Relaxed);
        if attempt == 0 {
            return Err(Status::unavailable("ambiguous prune failure"));
        }
        Ok(Response::new(PruneResponse {
            resource: request.into_inner().resource,
            space_reclaimed: 42,
            images_deleted: vec!["old".into()],
            ..Default::default()
        }))
    }

    type StreamPlatformStatsStream = StatsStream;

    async fn stream_platform_stats(
        &self,
        _: Request<PlatformStatsRequest>,
    ) -> Result<Response<Self::StreamPlatformStatsStream>, Status> {
        Ok(Response::new(Box::pin(futures_util::stream::pending())))
    }

    type StreamDaemonEventStream = EventStream;

    async fn stream_daemon_event(
        &self,
        request: Request<()>,
    ) -> Result<Response<Self::StreamDaemonEventStream>, Status> {
        if request.metadata().get_bin("x-signature-bin").is_none() {
            return Err(Status::unauthenticated("missing signature"));
        }
        Ok(Response::new(Box::pin(futures_util::stream::iter([Ok(
            DaemonEventResponse {
                kind: Some(daemon_event_response::Kind::DaemonResourceEventResponse(
                    DaemonResourceEventResponse {
                        r#type: 9,
                        action: "update".to_owned(),
                        resource_id: "service-1".to_owned(),
                    },
                )),
                ..Default::default()
            },
        )]))))
    }
}

#[tokio::test]
async fn platform_prune_is_signed_and_never_replayed_after_ambiguous_failure() {
    use citadel_platforms::prune::{PlatformPrunePort, PruneResource};
    let (address, attempts, _, stop) = start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[5; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancel = CancellationToken::new();
    assert!(client.prune(PruneResource::Image, &cancel).await.is_err());
    assert_eq!(
        attempts.load(Ordering::Relaxed),
        1,
        "A failed destructive request must not retry"
    );
    let result = client.prune(PruneResource::Image, &cancel).await.unwrap();
    assert_eq!(result.space_reclaimed, 42);
    assert_eq!(result.images_deleted, vec!["old"]);
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    stop.cancel();
}

fn platform_info() -> PlatformInfoResponse {
    PlatformInfoResponse {
        id: "daemon".to_owned(),
        agent_version: "fixture".to_owned(),
        api_version: "1.49".to_owned(),
        minimum_api_version: "1.41".to_owned(),
        platform_stat: Some(PlatformStatMessage {
            container_count: 2,
            containers_running: 1,
            containers_stopped: 1,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn start_fixture(
    behavior: HandshakeBehavior,
) -> (
    String,
    Arc<AtomicUsize>,
    Arc<Mutex<Vec<Vec<u8>>>>,
    CancellationToken,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let attempts = Arc::new(AtomicUsize::new(0));
    let nonces = Arc::new(Mutex::new(Vec::new()));
    let service = FixtureService {
        behavior,
        attempts: Arc::clone(&attempts),
        nonces: Arc::clone(&nonces),
    };
    let cancellation = CancellationToken::new();
    let server_cancellation = cancellation.clone();
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(PlatformServiceServer::new(service))
            .serve_with_shutdown(address, server_cancellation.cancelled_owned())
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (format!("http://{address}"), attempts, nonces, cancellation)
}

#[tokio::test]
async fn health_uses_the_signed_health_rpc_without_inventory_handshakes() {
    let (address, attempts, _, stop) = start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[5; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let token = CancellationToken::new();
    client.probe(&token).await.unwrap();
    client.probe(&token).await.unwrap();
    assert_eq!(attempts.load(Ordering::Relaxed), 0);
    token.cancel();
    assert_eq!(
        client.probe(&token).await.unwrap_err().kind,
        RuntimeErrorKind::Cancelled
    );
    stop.cancel();
}

#[tokio::test]
async fn retries_unavailable_with_a_fresh_signed_nonce() {
    let (address, attempts, nonces, server_cancellation) =
        start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[5; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancellation = CancellationToken::new();

    let info = client.get_info(&cancellation).await.unwrap();

    assert_eq!(info.daemon_id, "daemon");
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    let nonces = nonces.lock().await;
    assert_eq!(nonces.len(), 2);
    assert_ne!(nonces[0], nonces[1]);
    server_cancellation.cancel();
}

#[tokio::test]
async fn normalizes_a_unary_deadline_and_bounds_retries() {
    let (address, attempts, _, server_cancellation) = start_fixture(HandshakeBehavior::Delay).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[6; 32]),
        Duration::from_millis(25),
        true,
    )
    .await
    .unwrap();
    let cancellation = CancellationToken::new();

    let error = client.get_info(&cancellation).await.unwrap_err();

    assert_eq!(error.kind, RuntimeErrorKind::Timeout);
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
    server_cancellation.cancel();
}

#[tokio::test]
async fn persisted_agent_address_rebinds_without_sending_to_the_startup_endpoint() {
    let (old_address, old_attempts, _, old_stop) = start_fixture(HandshakeBehavior::FailOnce).await;
    let (new_address, new_attempts, _, new_stop) = start_fixture(HandshakeBehavior::FailOnce).await;
    let base = AgentClient::connect(
        &old_address,
        AgentRequestSigner::from_bytes(&[6; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let rebound = base.at_address(&new_address).unwrap();
    assert_eq!(base.address(), old_address);
    assert_eq!(rebound.address(), new_address);
    rebound.get_info(&CancellationToken::new()).await.unwrap();
    assert_eq!(old_attempts.load(Ordering::Relaxed), 0);
    assert_eq!(new_attempts.load(Ordering::Relaxed), 2);
    assert!(base.at_address("file:///var/run/docker.sock").is_err());
    old_stop.cancel();
    new_stop.cancel();
}

#[tokio::test]
async fn cancellation_ends_an_open_agent_stream() {
    let (address, _, _, server_cancellation) = start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[8; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancellation = CancellationToken::new();
    let mut stream = client
        .stream_stats(Duration::from_millis(10), &cancellation)
        .await
        .unwrap();

    cancellation.cancel();

    assert!(
        tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .is_none()
    );
    server_cancellation.cancel();
}

#[tokio::test]
async fn signed_agent_daemon_events_are_normalized_for_reconciliation() {
    let (address, _, _, server_cancellation) = start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[9; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancellation = CancellationToken::new();
    let mut stream = client.stream_daemon_events(&cancellation).await.unwrap();

    let event = stream.next().await.unwrap().unwrap();

    assert_eq!(event.resource_type, "service");
    assert_eq!(event.action, "update");
    assert!(stream.next().await.is_none());
    server_cancellation.cancel();
}

#[tokio::test]
async fn cancelled_task_inspection_does_not_issue_an_agent_request() {
    use citadel_platforms::SwarmTaskRuntimePort;
    let (address, _, _, server_cancellation) = start_fixture(HandshakeBehavior::FailOnce).await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[9; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        client
            .inspect_task("task", &cancellation)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::Cancelled
    );
    server_cancellation.cancel();
}

#[tokio::test]
async fn private_ca_is_required_preserved_on_retarget_and_checks_the_hostname() {
    let root = std::env::temp_dir().join(format!("citadel-agent-tls-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let output = std::process::Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost",
            "-addext",
            "basicConstraints=critical,CA:FALSE",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
        ])
        .current_dir(&root)
        .output()
        .expect("openssl is required for the Agent TLS test");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let certificate = std::fs::read(root.join("cert.pem")).unwrap();
    let key = std::fs::read(root.join("key.pem")).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let incoming = async_stream::stream! {
        loop { yield listener.accept().await.map(|(stream, _)| stream); }
    };
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let service = FixtureService {
        behavior: HandshakeBehavior::Delay,
        attempts: Arc::new(AtomicUsize::new(0)),
        nonces: Arc::new(Mutex::new(Vec::new())),
    };
    let base = AgentClient::lazy(
        "https://unused.invalid",
        AgentRequestSigner::from_bytes(&[5; 32]),
        Duration::from_secs(2),
        false,
    )
    .unwrap()
    .with_ca_certificate(Some(certificate.clone().into()))
    .unwrap();
    let server = tonic::transport::Server::builder()
        .tls_config(
            tonic::transport::ServerTlsConfig::new()
                .identity(tonic::transport::Identity::from_pem(&certificate, key)),
        )
        .unwrap()
        .add_service(PlatformServiceServer::new(service));
    let task = tokio::spawn(async move {
        server
            .serve_with_incoming_shutdown(incoming, shutdown.cancelled())
            .await
            .unwrap();
    });
    let address = format!("https://localhost:{port}");
    let untrusted = AgentClient::lazy(
        &address,
        AgentRequestSigner::from_bytes(&[5; 32]),
        Duration::from_secs(2),
        false,
    )
    .unwrap();
    assert!(untrusted.get_info(&CancellationToken::new()).await.is_err());
    let trusted = base.at_address(&address).unwrap();
    trusted.get_info(&CancellationToken::new()).await.unwrap();
    base.for_address(&address, &CancellationToken::new())
        .await
        .unwrap()
        .get_info(&CancellationToken::new())
        .await
        .unwrap();
    assert!(
        base.at_address(&format!("https://127.0.0.1:{port}"))
            .unwrap()
            .get_info(&CancellationToken::new())
            .await
            .is_err()
    );
    assert!(
        base.at_address(&format!("http://localhost:{port}"))
            .is_err()
    );
    stop.cancel();
    task.await.unwrap();
}
