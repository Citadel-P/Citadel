use std::collections::BTreeMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_agent::{app::Agent, config::AgentConfig};
use ed25519_dalek::SigningKey;
use tokio_util::sync::CancellationToken;

fn settings(edge: bool) -> BTreeMap<String, String> {
    let mut values = BTreeMap::from([(
        "HUB_PUBLIC_KEY".into(),
        STANDARD.encode(SigningKey::from_bytes(&[7; 32]).verifying_key().as_bytes()),
    )]);
    if edge {
        values.insert("CITADEL_AGENT_MODE".into(), "edge".into());
        values.insert("CITADEL_CORE_URL".into(), "https://core.example.com".into());
    }
    values
}

struct EdgeFiles(PathBuf);
impl EdgeFiles {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("citadel-agent-state-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn settings(&self) -> BTreeMap<String, String> {
        let mut values = settings(true);
        values.insert(
            "CITADEL_EDGE_ENROLLMENT_TOKEN".into(),
            "fixture-token".into(),
        );
        values.insert(
            "CITADEL_EDGE_AGENT_KEY_PATH".into(),
            self.0.join("agent.key").display().to_string(),
        );
        values.insert(
            "CITADEL_EDGE_IDENTITY_PATH".into(),
            self.0.join("identity.json").display().to_string(),
        );
        values
    }
}
impl Drop for EdgeFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn config(values: &BTreeMap<String, String>) -> AgentConfig {
    let mut config = AgentConfig::from_lookup(|key| values.get(key).cloned()).unwrap();
    config.port = 0; // The harness binds an ephemeral port; operator port zero is invalid.
    config
}

async fn probe(values: &BTreeMap<String, String>, port: u16) -> bool {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_citadel-agent"))
        .env_clear()
        .envs(values)
        .env("CITADEL_AGENT_PORT", port.to_string())
        .arg("healthcheck")
        .output()
        .await
        .unwrap()
        .status
        .success()
}

#[test]
fn executable_reports_compile_time_version_without_runtime_configuration() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_citadel-agent"))
        .env_clear()
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("citadel-agent {}", citadel_agent::INFORMATIONAL_VERSION)
    );
    assert_eq!(
        citadel_agent::VERSION,
        citadel_agent::INFORMATIONAL_VERSION
            .split('+')
            .next()
            .unwrap()
    );
}

#[tokio::test]
async fn health_is_unsigned_and_shutdown_releases_the_listener_in_both_modes() {
    for edge in [false, true] {
        let files = EdgeFiles::new();
        let values = if edge {
            files.settings()
        } else {
            settings(false)
        };
        let agent = Agent::bind(config(&values)).await.unwrap();
        let address = agent.local_addr().unwrap();
        assert_eq!(address.ip().is_loopback(), edge);
        let stop = CancellationToken::new();
        let server = tokio::spawn(agent.serve(stop.clone()));
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let base = format!("http://127.0.0.1:{}", address.port());
        let health = client.get(format!("{base}/health")).send().await.unwrap();
        assert_eq!(health.status(), 200);
        assert_eq!(
            health.json::<serde_json::Value>().await.unwrap()["Status"],
            "Healthy"
        );
        assert!(probe(&values, address.port()).await);
        assert_eq!(
            client
                .post(format!(
                    "{base}/citadel.containers.v1.ContainerService/Start"
                ))
                .send()
                .await
                .unwrap()
                .status(),
            if edge { 404 } else { 200 }
        );
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            TcpListener::bind(address).is_ok(),
            "listener must be released"
        );
        assert!(!probe(&values, address.port()).await);
    }
}

struct Certificates(PathBuf);
impl Certificates {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("citadel-agent-tls-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&directory).unwrap();
        let fixture = Self(directory);
        let result = std::process::Command::new("openssl")
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
            .current_dir(&fixture.0)
            .output()
            .expect("openssl is required for TLS tests");
        assert!(result.status.success(), "certificate generation failed");
        fixture
    }

    fn tls_settings(&self) -> BTreeMap<String, String> {
        let mut values = settings(false);
        values.insert("CITADEL_AGENT_TLS_MODE".into(), "Direct".into());
        values.insert(
            "CITADEL_AGENT_TLS_CERTIFICATE_PATH".into(),
            self.0.join("cert.pem").to_str().unwrap().into(),
        );
        values.insert(
            "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH".into(),
            self.0.join("key.pem").to_str().unwrap().into(),
        );
        values
    }
}
impl Drop for Certificates {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn direct_tls_serves_health_and_rejects_untrusted_clients() {
    let fixture = Certificates::new();
    let agent = Agent::bind(config(&fixture.tls_settings())).await.unwrap();
    let port = agent.local_addr().unwrap().port();
    let stop = CancellationToken::new();
    let server = tokio::spawn(agent.serve(stop.clone()));
    let url = format!("https://localhost:{port}/health");
    let certificate =
        reqwest::Certificate::from_pem(&std::fs::read(fixture.0.join("cert.pem")).unwrap())
            .unwrap();
    let trusted = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .add_root_certificate(certificate)
        .build()
        .unwrap();
    assert_eq!(trusted.get(&url).send().await.unwrap().status(), 200);
    assert!(probe(&fixture.tls_settings(), port).await);
    let untrusted = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    assert!(untrusted.get(&url).send().await.is_err());
    assert!(
        trusted
            .get(format!("https://127.0.0.1:{port}/health"))
            .send()
            .await
            .is_err(),
        "hostname verification remains enabled"
    );
    let channel = tonic::transport::Channel::from_shared(format!("https://localhost:{port}"))
        .unwrap()
        .tls_config(tonic::transport::ClientTlsConfig::new().ca_certificate(
            tonic::transport::Certificate::from_pem(
                std::fs::read(fixture.0.join("cert.pem")).unwrap(),
            ),
        ))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut rpc =
        citadel_contracts::citadel::volumes::v1::volume_service_client::VolumeServiceClient::new(
            channel,
        );
    assert_eq!(
        rpc.create(citadel_contracts::citadel::volumes::v1::CreateVolumeRequest::default())
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn invalid_tls_material_fails_before_binding_and_ca_requires_real_certificates() {
    let fixture = Certificates::new();
    let other = Certificates::new();
    let mut values = fixture.tls_settings();
    values.insert(
        "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH".into(),
        other.0.join("key.pem").to_str().unwrap().into(),
    );
    assert!(
        Agent::bind(config(&values)).await.is_err(),
        "mismatched key rejected"
    );
    std::fs::write(fixture.0.join("key.pem"), "not a private key").unwrap();
    assert!(Agent::bind(config(&fixture.tls_settings())).await.is_err());
    std::fs::remove_file(fixture.0.join("key.pem")).unwrap();
    assert!(Agent::bind(config(&fixture.tls_settings())).await.is_err());

    let files = EdgeFiles::new();
    let mut values = files.settings();
    values.insert(
        "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH".into(),
        fixture.0.join("cert.pem").to_str().unwrap().into(),
    );
    assert!(Agent::bind(config(&values)).await.is_ok());
    for pem in [
        "",
        "not a certificate",
        "-----BEGIN CERTIFICATE-----\nbm90LWRlcg==\n-----END CERTIFICATE-----",
    ] {
        std::fs::write(fixture.0.join("cert.pem"), pem).unwrap();
        assert!(
            Agent::bind(config(&values)).await.is_err(),
            "invalid CA rejected"
        );
    }
}

#[tokio::test]
async fn tcp_endpoint_can_be_composed_without_contacting_or_replacing_the_daemon() {
    let files = EdgeFiles::new();
    let mut values = files.settings();
    values.insert("DOCKER_HOST".into(), "tcp://127.0.0.1:2375".into());
    let agent = Agent::bind(config(&values)).await.unwrap();
    assert!(agent.local_addr().unwrap().ip().is_loopback());
}

#[cfg(unix)]
#[tokio::test]
async fn executable_handles_sigterm_without_a_database_or_docker_daemon() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let files = EdgeFiles::new();
    let mut values = files.settings();
    values.insert("CITADEL_AGENT_PORT".into(), port.to_string());
    values.insert(
        "DOCKER_HOST".into(),
        "unix:///nonexistent/citadel-test.sock".into(),
    );
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_citadel-agent"))
        .env_clear()
        .envs(values)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            assert!(
                child.try_wait().unwrap().is_none(),
                "Agent exited before health became available"
            );
            if client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        std::process::Command::new("kill")
            .args(["-TERM", &child.id().unwrap().to_string()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
