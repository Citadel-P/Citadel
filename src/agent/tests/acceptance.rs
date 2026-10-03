//! Black-box acceptance: all resource changes go through the packaged Core HTTP API.
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};
use std::{process::Command, time::Duration};
use uuid::Uuid;

#[path = "acceptance/swarm.rs"]
mod swarm;

const PASSWORD: &str = "citadel-acceptance-fixture-password";
const DIND: &str = "docker@sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0";
const POSTGRES: &str =
    "postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44";
const REGISTRY: &str =
    "registry@sha256:325b4b29b041e82803abeb703e201655e4e23ab83264ec1a7c9ddb0a5b14a6e0";

async fn docker(args: &[&str]) -> String {
    let output = tokio::time::timeout(
        Duration::from_secs(120),
        tokio::process::Command::new("docker")
            .args(args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("Docker command timed out")
    .unwrap();
    assert!(
        output.status.success(),
        "docker {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct Fixture {
    name: String,
    containers: Vec<String>,
    volumes: Vec<String>,
    client: Client,
    base: String,
    token: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in self.containers.iter().rev() {
            if std::thread::panicking()
                && name.contains("-swarm-worker-")
                && let Ok(ids) = Command::new("docker")
                    .args([
                        "exec",
                        name,
                        "docker",
                        "ps",
                        "-aq",
                        "--filter",
                        "label=com.citadel.system-role=swarm-node-agent",
                    ])
                    .output()
            {
                for id in String::from_utf8_lossy(&ids.stdout).lines() {
                    if let Ok(logs) = Command::new("docker")
                        .args(["exec", name, "docker", "logs", "--tail", "30", id])
                        .output()
                    {
                        eprintln!(
                            "{name} node-agent {id}:\n{}\n{}",
                            String::from_utf8_lossy(&logs.stdout),
                            String::from_utf8_lossy(&logs.stderr)
                        );
                    }
                }
            }
            if std::thread::panicking()
                && let Ok(logs) = Command::new("docker")
                    .args(["logs", "--tail", "60", name])
                    .output()
            {
                eprintln!(
                    "{name}:\n{}\n{}",
                    String::from_utf8_lossy(&logs.stdout),
                    String::from_utf8_lossy(&logs.stderr)
                );
            }
            let _ = Command::new("docker").args(["rm", "-fv", name]).output();
        }
        for volume in &self.volumes {
            let _ = Command::new("docker")
                .args(["volume", "rm", volume])
                .output();
        }
        let _ = Command::new("docker")
            .args(["network", "rm", &self.name])
            .output();
    }
}
impl Fixture {
    fn new() -> Self {
        Self {
            name: format!("citadel-agent-acceptance-{}", Uuid::now_v7().simple()),
            containers: vec![],
            volumes: vec![],
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(360))
                .build()
                .unwrap(),
            base: String::new(),
            token: String::new(),
        }
    }
    fn container(&self, role: &str) -> String {
        format!("{}-{role}", self.name)
    }
    async fn run(&mut self, role: &str, args: &[&str]) {
        let name = self.container(role);
        if !self.containers.contains(&name) {
            self.containers.push(name.clone());
        }
        let mut command = vec![
            "run",
            "-d",
            "--name",
            &name,
            "--network",
            &self.name,
            "--network-alias",
            role,
        ];
        command.extend_from_slice(args);
        docker(&command).await;
    }
    fn volume(&mut self, role: &str) -> String {
        let volume = self.container(role);
        self.volumes.push(volume.clone());
        format!("{volume}:/app/data")
    }
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = self.client.request(method, format!("{}{path}", self.base));
        if !self.token.is_empty() {
            request = request.bearer_auth(&self.token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let text = response.text().await.unwrap();
        let value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text)
                .unwrap_or_else(|_| panic!("{path} returned {status}: {text}"))
        };
        (status, value)
    }
    async fn api(&self, method: Method, path: &str, body: Option<Value>) -> Value {
        let (status, value) = self.request(method, path, body).await;
        assert!(status.is_success(), "{path}: {status}: {value}");
        value
    }
    async fn wait(&self, path: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
        loop {
            let (status, value) = self.request(Method::GET, path, None).await;
            if status.is_success() && predicate(&value) {
                return value;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "Timed out waiting for {path}: {status}: {value}"
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    async fn ready(&self) {
        tokio::time::timeout(Duration::from_secs(120), async {
            loop {
                if let Ok(response) = self
                    .client
                    .get(format!("{}/api/v1/setup/status", self.base))
                    .send()
                    .await
                    && response.status().is_success()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .expect("Core did not become ready");
    }
    async fn daemon(&mut self, role: &str) {
        let hostname = self.container(role);
        // Runtime PID files must disappear on restart, while Docker data persists.
        self.run(
            role,
            &[
                "--privileged",
                "--tmpfs",
                "/run",
                "--hostname",
                role,
                "-e",
                "DOCKER_TLS_CERTDIR=",
                DIND,
                "--tls=false",
                "--host=tcp://0.0.0.0:2375",
                "--host=unix:///var/run/docker.sock",
                "--insecure-registry=registry:5000",
            ],
        )
        .await;
        self.daemon_ready(role).await;
        docker(&["exec", &hostname, "docker", "pull", "alpine:3.24"]).await;
    }
    async fn daemon_ready(&self, role: &str) {
        let hostname = self.container(role);
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let result = tokio::process::Command::new("docker")
                    .args(["exec", &hostname, "docker", "info"])
                    .kill_on_drop(true)
                    .output()
                    .await
                    .unwrap();
                if result.status.success() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .expect("Fixture Docker daemon did not become ready");
    }
    async fn agent_ready(&self, role: &str) {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if docker(&[
                    "inspect",
                    "--format",
                    "{{.State.Health.Status}}",
                    &self.container(role),
                ])
                .await
                    == "healthy"
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .expect("Agent health check");
    }
    async fn route(&self, platform: &str) {
        // Only the remote daemon is reachable through this Agent. Core has no Docker socket.
        self.api(
            Method::POST,
            &format!("/api/v1/platforms/{platform}/prune"),
            Some(json!({"resource":"Build"})),
        )
        .await;
    }
    async fn workload(&self, platform: &str, profile: &str) -> String {
        let name = format!("acceptance-{profile}");
        let network = self
            .api(
                Method::POST,
                "/api/v1/networks",
                Some(json!({
                    "platformId":platform, "name":name, "driver":"bridge", "scope":"local"
                })),
            )
            .await;
        let network_path = format!(
            "/api/v1/networks/{platform}/{}",
            network["id"].as_str().unwrap()
        );
        assert_eq!(
            self.api(Method::GET, &network_path, None).await["name"],
            name
        );
        self.api(
            Method::DELETE,
            "/api/v1/networks",
            Some(json!({"platformId":platform,"ids":[network["id"]]})),
        )
        .await;
        assert_eq!(
            self.request(Method::GET, &network_path, None).await.0,
            StatusCode::NOT_FOUND
        );
        let networks = self
            .api(Method::GET, &format!("/api/v1/networks/{platform}"), None)
            .await;
        assert!(
            networks["networks"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["id"] != network["id"])
        );
        let volume = self
            .api(
                Method::POST,
                "/api/v1/volumes",
                Some(json!({
                    "platformId":platform, "name":name, "driver":"local"
                })),
            )
            .await;
        assert_eq!(volume["name"], name);
        let volume_path = format!("/api/v1/volumes/{platform}/{name}");
        assert_eq!(
            self.api(Method::GET, &volume_path, None).await["name"],
            name
        );
        self.api(
            Method::DELETE,
            "/api/v1/volumes",
            Some(json!({"platformId":platform,"names":[name]})),
        )
        .await;
        assert_eq!(
            self.request(Method::GET, &volume_path, None).await.0,
            StatusCode::NOT_FOUND
        );
        let volumes = self
            .api(Method::GET, &format!("/api/v1/volumes/{platform}"), None)
            .await;
        assert!(
            volumes["volumes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["name"] != name)
        );
        let stack = self.api(Method::POST, "/api/v1/stacks", Some(json!({
            "name":name, "platformId":platform, "stackSource":"WebEditor", "tagIds":[],
            "spec":{"$type":"WebEditor", "composeFile":"services:\n  runtime:\n    image: alpine:3.24\n    command: ['sh', '-c', 'echo citadel-acceptance; exec sleep 600']\n    stop_grace_period: 1s\n",
                "updateBehavior":"Disabled", "projectName":name, "destroyBeforeDeploy":true, "buildImageBindings":[]}
        }))).await;
        let id = stack["id"].as_str().unwrap().to_owned();
        let events = self
            .api(
                Method::POST,
                "/api/v1/stacks/apply",
                Some(json!({"id":id,"recreate":false})),
            )
            .await;
        let completed = events
            .as_array()
            .expect("stack apply progress array")
            .last()
            .unwrap();
        assert_eq!(completed["exitCode"], 0, "{events}");
        assert_eq!(completed["stackStatus"], "Healthy", "{events}");
        self.running(&id, "Running").await;
        self.api(Method::POST, "/api/v1/stacks/stop", Some(json!([id])))
            .await;
        self.running(&id, "Exited").await;
        self.api(Method::POST, "/api/v1/stacks/start", Some(json!([id])))
            .await;
        self.running(&id, "Running").await;
        // Inventory is read from Core persistence, populated by the runtime workers.
        let inventory = self
            .wait(
                &format!("/api/v1/platforms/{platform}/containers"),
                |body| {
                    body["containers"]
                        .as_array()
                        .is_some_and(|items| items.iter().any(|item| item["stackId"] == id))
                },
            )
            .await;
        let container = inventory["containers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["stackId"] == id)
            .unwrap();
        self.api(
            Method::GET,
            &format!(
                "/api/v1/containers/{}/inspect",
                container["id"].as_str().unwrap()
            ),
            None,
        )
        .await;
        self.api(
            Method::PATCH,
            "/api/v1/containers/pause",
            Some(json!([container["id"]])),
        )
        .await;
        self.running(&id, "Paused").await;
        self.api(
            Method::PATCH,
            "/api/v1/containers/unpause",
            Some(json!([container["id"]])),
        )
        .await;
        self.running(&id, "Running").await;
        id
    }
    async fn running(&self, stack: &str, state: &str) {
        self.wait(&format!("/api/v1/stacks/{stack}/data"), |body| {
            body["containers"]
                .as_array()
                .is_some_and(|items| items.len() == 1 && items[0]["state"] == state)
        })
        .await;
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_ACCEPTANCE_CORE_IMAGE and CITADEL_ACCEPTANCE_AGENT_IMAGE; creates isolated containers"]
async fn packaged_core_routes_direct_and_edge_resources_and_recovers_connections() {
    let core = std::env::var("CITADEL_ACCEPTANCE_CORE_IMAGE").unwrap();
    let agent = std::env::var("CITADEL_ACCEPTANCE_AGENT_IMAGE").unwrap();
    docker(&["image", "inspect", &core]).await;
    docker(&["image", "inspect", &agent]).await;
    let mut f = Fixture::new();
    tokio::time::timeout(Duration::from_secs(1500), exercise(&mut f, core, agent))
        .await
        .expect("Core/Agent acceptance exceeded its time budget");
}

async fn exercise(f: &mut Fixture, core: String, agent: String) {
    docker(&["network", "create", &f.name]).await;
    let gateway = docker(&[
        "network",
        "inspect",
        "--format",
        "{{(index .IPAM.Config 0).Gateway}}",
        &f.name,
    ])
    .await;
    // Reserve the next address after the registry and database. An IP address
    // also lets nested Swarm tasks reach Core without relying on outer DNS.
    let gateway: std::net::Ipv4Addr = gateway.parse().unwrap();
    let core_ip = std::net::Ipv4Addr::from(u32::from(gateway) + 3).to_string();
    let core_url = format!("http://{core_ip}:8001");
    f.run("registry", &[REGISTRY]).await;
    f.run(
        "postgres",
        &[
            "--tmpfs",
            "/var/lib/postgresql",
            "-e",
            "POSTGRES_USER=citadel_test",
            "-e",
            "POSTGRES_PASSWORD=fixture",
            "-e",
            "POSTGRES_DB=citadel_test",
            POSTGRES,
        ],
    )
    .await;
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let result = tokio::process::Command::new("docker")
                .args([
                    "exec",
                    &f.container("postgres"),
                    "pg_isready",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "citadel_test",
                ])
                .kill_on_drop(true)
                .output()
                .await
                .unwrap();
            if result.status.success() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("PostgreSQL did not become ready");
    let core_data = f.volume("core-data");
    f.run(
        "core",
        &[
            "--ip",
            &core_ip,
            "-p",
            "127.0.0.1::8000",
            "-v",
            &core_data,
            "-e",
            "DATABASE_URL=postgres://citadel_test:fixture@postgres:5432/citadel_test",
            "-e",
            "Transport__Mode=Disabled",
            "-e",
            "Transport__PublicUrl=http://localhost:8000",
            "-e",
            &format!("EdgeAgent__PublicGrpcUrl={core_url}"),
            "-e",
            "CITADEL_EDGE_AGENT_IMAGE=registry:5000/citadel-agent:acceptance",
            "-e",
            "AllowedHosts=*",
            "-e",
            "AgentTransport__AllowInsecure=true",
            "-e",
            "Automations__Enabled=false",
            "-e",
            "JobConfiguration__MonitoringInterval=1",
            "-e",
            "JobConfiguration__SwarmReconciliationIntervalSeconds=5",
            "-e",
            "JobConfiguration__FlashInterval=1",
            &core,
        ],
    )
    .await;
    let port = docker(&["port", &f.container("core"), "8000/tcp"]).await;
    f.base = format!("http://{port}");
    f.ready().await;
    let setup = f.api(Method::GET, "/api/v1/setup/status", None).await;
    assert_eq!(setup["requiresSetup"], true);
    let session = f.api(Method::POST, "/api/v1/setup/initialize", Some(json!({"name":"acceptance", "email":"acceptance@example.test", "password":PASSWORD}))).await;
    f.token = session["accessToken"].as_str().unwrap().into();
    let public_key = f
        .api(Method::GET, "/api/v1/platforms/agent/setup", None)
        .await["hubPublicKey"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!public_key.is_empty());

    f.daemon("direct-daemon").await;
    let wrong_key =
        citadel_adapters::connectors::agent::client::AgentRequestSigner::from_bytes(&[9; 32])
            .public_key_base64();
    f.run(
        "invalid",
        &[
            "-e",
            &format!("HUB_PUBLIC_KEY={wrong_key}"),
            "-e",
            "DOCKER_HOST=tcp://direct-daemon:2375",
            "--health-interval=1s",
            &agent,
        ],
    )
    .await;
    f.agent_ready("invalid").await;
    let (status, problem) = f.request(Method::POST, "/api/v1/platforms", Some(json!({"name":"invalid-agent", "address":"http://invalid:9000", "type":"Docker", "connectorType":"Agent", "tagIds":[]}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{problem}");
    f.run(
        "direct",
        &[
            "-e",
            &format!("HUB_PUBLIC_KEY={public_key}"),
            "-e",
            "DOCKER_HOST=tcp://direct-daemon:2375",
            "--health-interval=1s",
            &agent,
        ],
    )
    .await;
    f.agent_ready("direct").await;
    let platform = f.api(Method::POST, "/api/v1/platforms", Some(json!({"name":"acceptance-direct", "address":"http://direct:9000", "type":"Docker", "connectorType":"Agent", "tagIds":[]}))).await;
    let direct_id = platform["id"].as_str().unwrap().to_owned();
    assert_eq!(platform["status"], "Online");
    assert!(!platform["agentVersion"].as_str().unwrap().is_empty());
    f.route(&direct_id).await;
    let direct_stack = f.workload(&direct_id, "direct").await;
    docker(&["stop", "-t", "12", &f.container("direct")]).await;
    f.wait(&format!("/api/v1/platforms/{direct_id}"), |v| {
        v["status"] == "Offline"
    })
    .await;
    let (status, problem) = f
        .request(
            Method::POST,
            &format!("/api/v1/platforms/{direct_id}/prune"),
            Some(json!({"resource":"Build"})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{problem}");
    assert_eq!(
        problem["type"],
        "https://citadel.dev/problems/platform_unavailable"
    );
    docker(&["start", &f.container("direct")]).await;
    f.wait(&format!("/api/v1/platforms/{direct_id}"), |v| {
        v["status"] == "Online"
    })
    .await;
    f.route(&direct_id).await;
    f.running(&direct_stack, "Running").await;
    println!(
        "Direct: HTTP setup, signing rejection, registration, stacks, inventory and offline recovery passed"
    );

    f.daemon("edge-daemon").await;
    let platform = f.api(Method::POST, "/api/v1/platforms", Some(json!({"name":"acceptance-edge", "type":"Docker", "connectorType":"EdgeAgent", "tagIds":[]}))).await;
    let edge_id = platform["id"].as_str().unwrap().to_owned();
    assert_eq!(platform["platformDescriptor"]["$type"], "Docker");
    let enrollment = f
        .api(
            Method::POST,
            &format!("/api/v1/platforms/{edge_id}/edge/enrollments"),
            None,
        )
        .await;
    assert_eq!(enrollment["instructions"]["coreUrl"], core_url);
    let edge_data = f.volume("edge-data");
    let mut args = vec![
        "-e",
        "CITADEL_AGENT_MODE=edge",
        "-e",
        "CITADEL_CORE_URL=http://core:8001",
        "-e",
        "DOCKER_HOST=tcp://edge-daemon:2375",
        "-v",
        &edge_data,
    ];
    let credential = format!(
        "CITADEL_EDGE_ENROLLMENT_TOKEN={}",
        enrollment["token"].as_str().unwrap()
    );
    let mut initial = args.clone();
    initial.extend(["-e", &credential, &agent]);
    f.run("edge", &initial).await;
    let status_path = format!("/api/v1/platforms/{edge_id}/edge/status");
    let connected = f
        .wait(&status_path, |v| v["connectionStatus"] == "Connected")
        .await;
    let fingerprint = connected["agentFingerprint"].clone();
    assert!(fingerprint.is_string());
    f.wait(&format!("/api/v1/platforms/{edge_id}"), |v| {
        v["status"] == "Online"
    })
    .await;
    f.route(&edge_id).await;
    let edge_stack = f.workload(&edge_id, "edge").await;
    docker(&["stop", "-t", "12", &f.container("edge")]).await;
    f.wait(&status_path, |v| v["connectionStatus"] == "Offline")
        .await;
    let (status, problem) = f
        .request(
            Method::POST,
            &format!("/api/v1/platforms/{edge_id}/prune"),
            Some(json!({"resource":"Build"})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{problem}");
    assert_eq!(
        problem["type"],
        "https://citadel.dev/problems/platform_unavailable"
    );
    docker(&["rm", "-fv", &f.container("edge")]).await;
    args.push(&agent);
    f.run("edge", &args).await;
    let reconnected = f
        .wait(&status_path, |v| v["connectionStatus"] == "Connected")
        .await;
    assert_eq!(reconnected["agentFingerprint"], fingerprint);
    f.wait(&format!("/api/v1/platforms/{edge_id}"), |v| {
        v["status"] == "Online"
    })
    .await;
    f.route(&edge_id).await;
    f.running(&edge_stack, "Running").await;

    // Restart the full Core process: verify persisted resources, credentials and worker recovery.
    docker(&["restart", "-t", "12", &f.container("core")]).await;
    // Docker may allocate a different random host port when restarting.
    let port = docker(&["port", &f.container("core"), "8000/tcp"]).await;
    f.base = format!("http://{port}");
    f.ready().await;
    f.wait(&status_path, |v| {
        v["connectionStatus"] == "Connected"
            && v["lastConnectedAtUtc"] != reconnected["lastConnectedAtUtc"]
    })
    .await;
    for id in [&direct_id, &edge_id] {
        f.wait(&format!("/api/v1/platforms/{id}"), |v| {
            v["status"] == "Online"
        })
        .await;
    }
    f.route(&direct_id).await;
    f.route(&edge_id).await;
    f.running(&direct_stack, "Running").await;
    f.running(&edge_stack, "Running").await;
    f.api(
        Method::DELETE,
        "/api/v1/stacks",
        Some(json!([direct_stack, edge_stack])),
    )
    .await;
    for profile in ["direct", "edge"] {
        let remaining = docker(&[
            "exec",
            &f.container(&format!("{profile}-daemon")),
            "docker",
            "ps",
            "-aq",
            "--filter",
            &format!("label=com.docker.compose.project=acceptance-{profile}"),
        ])
        .await;
        assert!(
            remaining.is_empty(),
            "Deleted {profile} Stack still has containers: {remaining}"
        );
    }
    f.api(
        Method::POST,
        &format!("/api/v1/platforms/{edge_id}/edge/revoke"),
        None,
    )
    .await;
    f.wait(&status_path, |v| v["connectionStatus"] == "Revoked")
        .await;
    let (status, problem) = f
        .request(
            Method::POST,
            &format!("/api/v1/platforms/{edge_id}/prune"),
            Some(json!({"resource":"Build"})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{problem}");
    assert_eq!(
        problem["type"],
        "https://citadel.dev/problems/platform_unavailable"
    );
    println!(
        "Edge: HTTP enrollment, stacks, token-free reconnect, revocation and Core restart persistence passed"
    );
    swarm::exercise(f, &agent, &public_key, &core_url).await;
    if let Ok(baseline) = std::env::var("CITADEL_ACCEPTANCE_BASELINE_AGENT_IMAGE") {
        assert!(
            baseline
                .rsplit_once("@sha256:")
                .is_some_and(|(_, digest)| digest.len() == 64
                    && digest.bytes().all(|b| b.is_ascii_hexdigit())),
            "The released Agent baseline must be pinned by digest"
        );
        docker(&["image", "inspect", &baseline]).await;
        f.daemon("baseline-daemon").await;
        f.run(
            "baseline",
            &[
                "-e",
                &format!("HUB_PUBLIC_KEY={public_key}"),
                "-e",
                "DOCKER_HOST=tcp://baseline-daemon:2375",
                "--health-interval=1s",
                &baseline,
            ],
        )
        .await;
        f.agent_ready("baseline").await;
        let platform = f.api(Method::POST, "/api/v1/platforms", Some(json!({"name":"released-agent", "address":"http://baseline:9000", "type":"Docker", "connectorType":"Agent", "tagIds":[]}))).await;
        let id = platform["id"].as_str().unwrap();
        let stack = f.workload(id, "baseline").await;
        docker(&["restart", "-t", "12", &f.container("baseline")]).await;
        f.agent_ready("baseline").await;
        f.route(id).await;
        f.running(&stack, "Running").await;
        f.api(Method::DELETE, "/api/v1/stacks", Some(json!([stack])))
            .await;
        println!(
            "Released Agent baseline {baseline}: candidate Core registration, resource operations, stacks and restart passed"
        );
    } else {
        println!(
            "Mixed-version compatibility was not run: no released Agent digest was configured."
        );
    }
}
