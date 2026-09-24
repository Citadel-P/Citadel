use super::*;
use citadel_contracts::citadel::edge::v1::edge_agent_service_server::{
    EdgeAgentService, EdgeAgentServiceServer,
};
use ed25519_dalek::{Signature, VerifyingKey};
use futures_util::stream::BoxStream;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status, Streaming};

pub(super) fn config(profile: EdgeProfile) -> EdgeConfig {
    EdgeConfig {
        profile,
        core_url: "http://127.0.0.1:1".parse().unwrap(),
        enrollment_token: Some(zeroize::Zeroizing::new("fixture-token".into())),
        key_path: "unused-key".into(),
        identity_path: "unused-identity".into(),
        core_ca_path: None,
    }
}
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("citadel-edge-test-{}", Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn config(&self, profile: EdgeProfile) -> EdgeConfig {
        EdgeConfig {
            key_path: self.0.join("agent.key"),
            identity_path: self.0.join("identity.json"),
            ..config(profile)
        }
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn accepted(build: bool) -> SessionAccepted {
    let id = Uuid::now_v7().to_string();
    SessionAccepted {
        platform_id: if build {
            Uuid::nil().to_string()
        } else {
            id.clone()
        },
        resource_id: id,
        agent_id: Uuid::now_v7().to_string(),
        session_id: Uuid::now_v7().to_string(),
        resource_type: i32::from(build),
        node_id: String::new(),
    }
}
#[test]
fn state_survives_restart_is_private_and_preserves_existing_file_format() {
    for profile in [EdgeProfile::Ordinary, EdgeProfile::BuildPool] {
        let files = Files::new();
        let mut config = files.config(profile);
        let mut state = State::load(&config).unwrap();
        let key = state.key.to_bytes();
        let accepted = accepted(matches!(config.profile, EdgeProfile::BuildPool));
        state.accept(&config, &accepted).unwrap();
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config.identity_path).unwrap()).unwrap();
        assert_eq!(persisted["AgentId"], accepted.agent_id);
        assert!(
            persisted["AgentFingerprint"]
                .as_str()
                .unwrap()
                .starts_with("SHA256:")
        );
        assert_eq!(
            persisted["EnrollmentTokenFingerprint"],
            identity::fingerprint(b"fixture-token")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&config.key_path, &config.identity_path] {
                assert_eq!(
                    std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        config.enrollment_token = None;
        let state = State::load(&config).unwrap();
        assert_eq!(state.key.to_bytes(), key);
        assert_eq!(
            state.identity.unwrap().agent_id.to_string(),
            accepted.agent_id
        );
        assert_eq!(std::fs::read_dir(&files.0).unwrap().count(), 2);
    }
}
#[test]
fn concurrent_key_creation_uses_one_key_and_corruption_never_replaces_it() {
    let files = Files::new();
    let config = files.config(EdgeProfile::Ordinary);
    let keys = std::thread::scope(|s| {
        (0..8)
            .map(|_| s.spawn(|| State::load(&config).unwrap().key.to_bytes()))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|v| v.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(keys.iter().all(|v| v == &keys[0]));
    std::fs::write(&config.key_path, b"broken-key").unwrap();
    assert!(State::load(&config).is_err());
    assert_eq!(std::fs::read(&config.key_path).unwrap(), b"broken-key");
    std::fs::write(&config.key_path, keys[0]).unwrap();
    std::fs::write(&config.identity_path, b"broken-json").unwrap();
    assert!(State::load(&config).is_err());
    assert_eq!(
        std::fs::read(&config.identity_path).unwrap(),
        b"broken-json"
    );
    assert_eq!(std::fs::read(&config.key_path).unwrap(), keys[0]);
}
#[test]
fn token_changes_and_rejections_reset_only_the_intended_profiles() {
    let files = Files::new();
    let mut config = files.config(EdgeProfile::Ordinary);
    let mut state = State::load(&config).unwrap();
    state.accept(&config, &accepted(false)).unwrap();
    let key = state.key.to_bytes();
    config.enrollment_token = Some(zeroize::Zeroizing::new("replacement".into()));
    let mut next = State::load(&config).unwrap();
    assert!(next.identity.is_none());
    assert_ne!(next.key.to_bytes(), key);
    next.accept(&config, &accepted(false)).unwrap();
    next.rejected(&config, "not active").unwrap();
    assert!(next.identity.is_none());
    let target = accepted(false);
    let bootstrap = files.0.join("bootstrap");
    std::fs::write(&bootstrap, "first-bootstrap").unwrap();
    config.profile = EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
        bootstrap_file: bootstrap.clone(),
        platform_id: target.platform_id.clone(),
        service_id: "service".into(),
        task_id: "task".into(),
        node_id: "node".into(),
        node_hostname: "worker".into(),
        cluster_id: "cluster".into(),
    });
    let target = SessionAccepted {
        node_id: "node".into(),
        ..target
    };
    let mut state = State::load(&config).unwrap();
    state.accept(&config, &target).unwrap();
    let key = state.key.to_bytes();
    std::fs::write(&bootstrap, "rotated-bootstrap").unwrap();
    let mut state = State::load(&config).unwrap();
    assert_eq!(state.key.to_bytes(), key);
    state.rejected(&config, "temporary rejection").unwrap();
    assert!(state.identity.is_some());
    state.rejected(&config, "credentials revoked").unwrap();
    assert!(state.identity.is_none());
    assert_ne!(state.key.to_bytes(), key);
}
#[test]
fn invalid_acceptance_and_missing_key_preserve_persisted_identity() {
    let files = Files::new();
    let config = files.config(EdgeProfile::BuildPool);
    let mut state = State::load(&config).unwrap();
    let target = accepted(true);
    state.accept(&config, &target).unwrap();
    let before = std::fs::read(&config.identity_path).unwrap();
    for target in [
        SessionAccepted {
            session_id: "invalid".into(),
            ..target.clone()
        },
        accepted(false),
        SessionAccepted {
            resource_type: 99,
            ..target.clone()
        },
        SessionAccepted {
            agent_id: Uuid::nil().to_string(),
            ..target.clone()
        },
    ] {
        assert!(state.accept(&config, &target).is_err());
        assert_eq!(std::fs::read(&config.identity_path).unwrap(), before);
    }
    std::fs::remove_file(&config.key_path).unwrap();
    assert!(State::load(&config).is_err());
    assert!(!config.key_path.exists());
}
#[test]
fn backoff_jitter_stays_within_protocol_bounds() {
    for base in [1, 2, 16, 60] {
        for _ in 0..100 {
            let seconds = jitter(Duration::from_secs(base)).as_secs_f64();
            assert!((base as f64 * 0.8..=base as f64 * 1.2).contains(&seconds));
        }
    }
}

#[derive(Clone)]
struct Core {
    target: SessionAccepted,
    seen: mpsc::Sender<AgentEnvelope>,
    connections: Arc<AtomicUsize>,
    key: Arc<Mutex<Option<[u8; 32]>>>,
    reject_first_hello: bool,
}
#[tonic::async_trait]
impl EdgeAgentService for Core {
    type ConnectStream = BoxStream<'static, Result<CoreEnvelope, Status>>;
    async fn connect(
        &self,
        request: Request<Streaming<AgentEnvelope>>,
    ) -> Result<Response<Self::ConnectStream>, Status> {
        let this = self.clone();
        let mut incoming = request.into_inner();
        let attempt = this.connections.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let first=incoming.message().await?.unwrap();
            this.seen.send(first.clone()).await.unwrap();
            match first.body.unwrap() {
                AgentBody::EnrollmentRequest(r)=>{ *this.key.lock().unwrap()=Some(r.public_key.try_into().unwrap()); },
                AgentBody::Hello(_)=>{
                    if this.reject_first_hello && attempt==1 {
                        yield core_message("",CoreBody::SessionRejected(SessionRejected{reason:"revoked".into()})); return;
                    }
                    let nonce=vec![42;32]; let time=chrono::Utc::now().timestamp();
                    yield core_message("",CoreBody::AuthChallenge(AuthChallenge{nonce:nonce.clone(),timestamp_unix_seconds:time}));
                    let response=incoming.message().await?.unwrap();
                    let Some(AgentBody::AuthChallengeResponse(r))=&response.body else {panic!("challenge response required")};
                    assert_eq!(r.nonce,nonce); assert_eq!(r.timestamp_unix_seconds,time);
                    let key=VerifyingKey::from_bytes(&this.key.lock().unwrap().unwrap()).unwrap();
                    key.verify_strict(&[&time.to_le_bytes()[..],&nonce].concat(),&Signature::from_slice(&r.signature).unwrap()).unwrap();
                    this.seen.send(response).await.unwrap();
                }
                other=>panic!("unexpected opening: {other:?}"),
            }
            let target=SessionAccepted{session_id:Uuid::now_v7().to_string(),..this.target.clone()};
            yield core_message(&target.session_id,CoreBody::SessionAccepted(target.clone()));
            let Some(heartbeat)=incoming.message().await? else {return;}; assert_eq!(heartbeat.session_id,target.session_id);
            assert!(matches!(heartbeat.body,Some(AgentBody::Heartbeat(_)))); this.seen.send(heartbeat).await.unwrap();
            if attempt==0 {
                let mut command=core_message(&target.session_id,CoreBody::Command(EdgeCommand{command_id:Uuid::now_v7().to_string(),kind:0,..Default::default()}));
                if let Some(CoreBody::Command(c)) = &command.body {command.command_id=c.command_id.clone();} yield command;
                this.seen.send(incoming.message().await?.unwrap()).await.unwrap();
                yield core_message(&target.session_id,CoreBody::Disconnect(Disconnect{reason:"test reconnect".into()}));
            } else {
                while let Some(message)=incoming.message().await? { this.seen.send(message).await.unwrap(); }
            }
        })))
    }
}
fn core_message(session: &str, body: CoreBody) -> CoreEnvelope {
    CoreEnvelope {
        envelope_id: Uuid::now_v7().to_string(),
        session_id: session.into(),
        body: Some(body),
        ..Default::default()
    }
}
struct Harness {
    docker: DockerClient,
    docker_mode: Arc<AtomicUsize>,
    core: Core,
    seen: mpsc::Receiver<AgentEnvelope>,
    url: String,
    stop: CancellationToken,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl Harness {
    async fn start(build: bool, reject: bool) -> Self {
        Self::start_with_tls(build, reject, None).await
    }
    async fn start_with_tls(
        build: bool,
        reject: bool,
        tls: Option<tonic::transport::Identity>,
    ) -> Self {
        let stop = CancellationToken::new();
        let daemon = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", daemon.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(2),
            "/host",
        )
        .unwrap();
        let docker_mode = Arc::new(AtomicUsize::new(0));
        let mode = docker_mode.clone();
        let router = axum::Router::new().fallback(move |uri: axum::http::Uri| {
            let mode = mode.clone();
            async move {
                use axum::response::IntoResponse;
                match mode.load(Ordering::SeqCst) {
                    1 => return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
                    2 => tokio::time::sleep(Duration::from_secs(30)).await,
                    _ => {}
                }
                axum::Json(if uri.path() == "/version" {
                    serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41","Version":"29"})
                } else {
                    serde_json::json!({"ID":"daemon","Name":"fixture-host","ServerVersion":"29"})
                })
                .into_response()
            }
        });
        let daemon_stop = stop.clone();
        tokio::spawn(async move {
            axum::serve(daemon, router)
                .with_graceful_shutdown(daemon_stop.cancelled_owned())
                .await
                .unwrap();
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = if tls.is_some() {
            format!(
                "https://localhost:{}",
                listener.local_addr().unwrap().port()
            )
        } else {
            format!("http://{}", listener.local_addr().unwrap())
        };
        let (sender, seen) = mpsc::channel(32);
        let core = Core {
            target: accepted(build),
            seen: sender,
            connections: Arc::new(AtomicUsize::new(0)),
            key: Arc::new(Mutex::new(None)),
            reject_first_hello: reject,
        };
        let service = core.clone();
        let core_stop = stop.clone();
        tokio::spawn(async move {
            let mut server = tonic::transport::Server::builder();
            if let Some(identity) = tls {
                server = server
                    .tls_config(tonic::transport::ServerTlsConfig::new().identity(identity))
                    .unwrap();
            }
            server
                .add_service(EdgeAgentServiceServer::new(service))
                .serve_with_incoming_shutdown(
                    futures_util::stream::unfold(listener, |listener| async {
                        Some((listener.accept().await.map(|v| v.0), listener))
                    }),
                    core_stop.cancelled_owned(),
                )
                .await
                .unwrap();
        });
        Self {
            docker,
            docker_mode,
            core,
            seen,
            url,
            stop,
        }
    }
    async fn next(&mut self) -> AgentEnvelope {
        tokio::time::timeout(Duration::from_secs(8), self.seen.recv())
            .await
            .unwrap()
            .unwrap()
    }
}
#[tokio::test]
async fn enrollment_heartbeat_command_failure_and_challenged_reconnect_preserve_identity() {
    for build in [false, true] {
        let files = Files::new();
        let mut h = Harness::start(build, false).await;
        let mut config = files.config(if build {
            EdgeProfile::BuildPool
        } else {
            EdgeProfile::Ordinary
        });
        config.core_url = h.url.parse().unwrap();
        let agent = EdgeAgent::new(config.clone(), h.docker.clone())
            .await
            .unwrap();
        let stop = CancellationToken::new();
        let task = tokio::spawn(agent.run(stop.clone()));
        let Some(AgentBody::EnrollmentRequest(enrollment)) = h.next().await.body else {
            panic!("enrollment")
        };
        assert_eq!(enrollment.enrollment_token, "fixture-token");
        assert_eq!(enrollment.protocol_version, 2);
        assert_eq!(enrollment.agent_version, crate::VERSION);
        assert_eq!(enrollment.daemon_id, "daemon");
        assert_eq!(enrollment.hostname, "fixture-host");
        let Some(AgentBody::Heartbeat(heartbeat)) = h.next().await.body else {
            panic!("heartbeat")
        };
        assert!(heartbeat.docker_reachable);
        assert_eq!(heartbeat.docker_version, "29");
        assert_eq!(heartbeat.agent_version, crate::VERSION);
        let failure = h.next().await;
        assert!(Uuid::parse_str(&failure.command_id).is_ok());
        assert!(
            matches!(failure.body,Some(AgentBody::CommandFailed(v)) if v.code=="invalid_argument")
        );
        let Some(AgentBody::Hello(hello)) = h.next().await.body else {
            panic!("hello")
        };
        assert_eq!(hello.resource_type, i32::from(build));
        assert_eq!(hello.agent_id, h.core.target.agent_id);
        assert_eq!(
            hello.agent_fingerprint,
            identity::fingerprint(&enrollment.public_key)
        );
        assert!(matches!(
            h.next().await.body,
            Some(AgentBody::AuthChallengeResponse(_))
        ));
        assert!(matches!(h.next().await.body, Some(AgentBody::Heartbeat(_))));
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        config.enrollment_token = None;
        let restored = State::load(&config).unwrap();
        assert_eq!(
            restored.key.verifying_key().as_bytes().as_slice(),
            enrollment.public_key
        );
    }
}
#[tokio::test]
async fn identity_write_failure_stops_without_retrying_enrollment_or_replacing_key() {
    let files = Files::new();
    let h = Harness::start(false, false).await;
    let mut config = files.config(EdgeProfile::Ordinary);
    config.core_url = h.url.parse().unwrap();
    let agent = EdgeAgent::new(config.clone(), h.docker.clone())
        .await
        .unwrap();
    let key = std::fs::read(&config.key_path).unwrap();
    // Make the destination unwritable after startup without depending on Unix user privileges.
    std::fs::create_dir(&config.identity_path).unwrap();
    let error = tokio::time::timeout(Duration::from_secs(3), agent.run(CancellationToken::new()))
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("persist Edge identity"));
    assert_eq!(h.core.connections.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read(&config.key_path).unwrap(), key);
    assert!(config.identity_path.is_dir());
}

#[tokio::test]
async fn rejected_hello_reenrolls_with_a_new_key_but_transport_disconnect_does_not() {
    let files = Files::new();
    let mut h = Harness::start(false, true).await;
    let mut config = files.config(EdgeProfile::Ordinary);
    config.core_url = h.url.parse().unwrap();
    let agent = EdgeAgent::new(config, h.docker.clone()).await.unwrap();
    let stop = CancellationToken::new();
    let task = tokio::spawn(agent.run(stop.clone()));
    let Some(AgentBody::EnrollmentRequest(first)) = h.next().await.body else {
        panic!("enrollment")
    };
    h.next().await;
    h.next().await;
    assert!(matches!(h.next().await.body, Some(AgentBody::Hello(_))));
    let Some(AgentBody::EnrollmentRequest(next)) = h.next().await.body else {
        panic!("reenrollment")
    };
    assert_ne!(first.public_key, next.public_key);
    h.next().await;
    stop.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn custom_ca_allows_core_tls_without_disabling_hostname_or_default_trust() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let files = Files::new();
    let generated = std::process::Command::new("openssl")
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
        .current_dir(&files.0)
        .output()
        .expect("openssl required for TLS tests");
    assert!(generated.status.success());
    let cert = std::fs::read(files.0.join("cert.pem")).unwrap();
    let key = std::fs::read(files.0.join("key.pem")).unwrap();
    let mut h = Harness::start_with_tls(
        false,
        false,
        Some(tonic::transport::Identity::from_pem(cert, key)),
    )
    .await;
    let mut config = files.config(EdgeProfile::Ordinary);
    config.core_url = h.url.parse().unwrap();
    let mut untrusted = EdgeAgent::new(config.clone(), h.docker.clone())
        .await
        .unwrap();
    let mut accepted = false;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), untrusted.connection(&mut accepted))
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(h.core.connections.load(Ordering::SeqCst), 0);
    config.core_ca_path = Some(files.0.join("cert.pem"));
    let mut wrong = config.clone();
    wrong.core_url.set_host(Some("127.0.0.1")).unwrap();
    let mut wrong = EdgeAgent::new(wrong, h.docker.clone()).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), wrong.connection(&mut accepted))
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(h.core.connections.load(Ordering::SeqCst), 0);
    let mut trusted = EdgeAgent::new(config, h.docker.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), trusted.connection(&mut accepted))
        .await
        .unwrap()
        .unwrap();
    assert!(accepted);
    assert!(matches!(
        h.next().await.body,
        Some(AgentBody::EnrollmentRequest(_))
    ));
}

#[tokio::test]
async fn heartbeats_probe_docker_again_and_shutdown_interrupts_a_stalled_probe() {
    let files = Files::new();
    let mut h = Harness::start(false, false).await;
    let mut config = files.config(EdgeProfile::Ordinary);
    config.core_url = h.url.parse().unwrap();
    let mut agent = EdgeAgent::new(config, h.docker.clone()).await.unwrap();
    agent.heartbeat_interval = Duration::from_millis(100);
    let stop = CancellationToken::new();
    let task = tokio::spawn(agent.run(stop.clone()));
    let mut challenged = false;
    loop {
        let message = h.next().await;
        if matches!(message.body, Some(AgentBody::AuthChallengeResponse(_))) {
            challenged = true;
        }
        if challenged && matches!(message.body, Some(AgentBody::Heartbeat(_))) {
            break;
        }
    }
    h.docker_mode.store(1, Ordering::SeqCst);
    loop {
        if let Some(AgentBody::Heartbeat(v)) = h.next().await.body
            && !v.docker_reachable
        {
            assert!(v.docker_version.is_empty());
            assert_eq!(v.daemon_id, "daemon");
            break;
        }
    }
    h.docker_mode.store(2, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(200)).await;
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn swarm_opening_preserves_injected_fields_and_bootstrap_rotation() {
    let files = Files::new();
    let bootstrap = files.0.join("bootstrap");
    std::fs::write(&bootstrap, "first-token").unwrap();
    let platform = Uuid::now_v7().to_string();
    let config = files.config(EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
        bootstrap_file: bootstrap.clone(),
        platform_id: platform.clone(),
        service_id: "service".into(),
        task_id: "task".into(),
        node_id: "node".into(),
        node_hostname: "worker".into(),
        cluster_id: "cluster".into(),
    }));
    let docker = DockerClient::with_endpoint(
        "http://127.0.0.1:1".parse().unwrap(),
        Duration::from_secs(1),
        "/host",
    )
    .unwrap();
    let mut agent = EdgeAgent::new(config, docker).await.unwrap();
    std::fs::write(&bootstrap, "rotated-token").unwrap();
    let observed = Observation {
        daemon_id: "daemon".into(),
        hostname: "worker".into(),
        version: "29".into(),
        cluster_id: "cluster".into(),
        node_id: "node".into(),
        swarm_role: "worker".into(),
    };
    let Some(AgentBody::EnrollmentRequest(r)) = agent.opening(&observed).unwrap().body else {
        panic!("enrollment")
    };
    assert_eq!(r.enrollment_token, "rotated-token");
    assert_eq!(r.profile, 1);
    assert_eq!(
        (
            r.cluster_id.as_str(),
            r.node_id.as_str(),
            r.service_id.as_str(),
            r.task_id.as_str(),
            r.swarm_role.as_str()
        ),
        ("cluster", "node", "service", "task", "worker")
    );
    let accepted = SessionAccepted {
        platform_id: platform.clone(),
        resource_id: platform,
        node_id: "node".into(),
        ..accepted(false)
    };
    agent.state.accept(&agent.config, &accepted).unwrap();
    let Some(AgentBody::Hello(r)) = agent.opening(&observed).unwrap().body else {
        panic!("hello")
    };
    assert_eq!(r.profile, 1);
    assert_eq!(r.resource_type, 0);
    assert_eq!(r.node_id, "node");
    assert_eq!(r.platform_id, accepted.platform_id);
    assert_eq!(r.agent_id, accepted.agent_id);
}
