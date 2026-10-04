//! Real protocol operations on the disposable daemon supplied by test-agent-compatibility.sh.
#[path = "compatibility/swarm.rs"]
mod swarm;
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_adapters::{
    connectors::{
        agent::client::{AgentClient, AgentRequestSigner},
        edge::{EdgeIntake, EdgeRegistry, EdgeSession, EdgeTarget},
    },
    persistence::postgres::platforms::edge::store::PostgresEdgeStore,
};
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind as Kind, edge_agent_service_server::EdgeAgentServiceServer},
    images::v1::*,
    stacks::v1::*,
    swarm::v1::*,
};
use ed25519_dalek::{Signer, SigningKey};
use futures_util::{StreamExt, stream::BoxStream};
use prost::Message;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Status, metadata::MetadataValue, transport::Channel};
use uuid::Uuid;

const PASSWORD: &str = "compatibility-password";
const SECRET: &str = "compatibility-build-secret";

fn sign<T: Message>(value: T, path: &'static str) -> Request<T> {
    let time = chrono::Utc::now().timestamp().to_le_bytes();
    let nonce = *Uuid::now_v7().as_bytes();
    let hash = Sha256::digest(value.encode_to_vec());
    let signature = SigningKey::from_bytes(&[42; 32])
        .sign(&[&time[..], &nonce, path.as_bytes(), &hash].concat())
        .to_bytes();
    let mut request = Request::new(value);
    for (key, value) in [
        ("x-timestamp-bin", &time[..]),
        ("x-nonce-bin", &nonce),
        ("x-content-sha256-bin", &hash),
        ("x-signature-bin", &signature),
    ] {
        request
            .metadata_mut()
            .insert_bin(key, MetadataValue::from_bytes(value));
    }
    request
}

enum Wire {
    Direct(Channel),
    Edge(Arc<EdgeSession>),
}
impl Wire {
    async fn stream<Q, R>(
        &self,
        kind: Kind,
        path: &'static str,
        value: Q,
    ) -> BoxStream<'static, Result<R, Status>>
    where
        Q: Message + Default + Send + Sync + 'static,
        R: Message + Default + Send + Sync + 'static,
    {
        match self {
            Self::Direct(channel) => {
                let mut client = tonic::client::Grpc::new(channel.clone());
                client.ready().await.unwrap();
                client
                    .server_streaming(
                        sign(value, path),
                        axum::http::uri::PathAndQuery::from_static(path),
                        tonic_prost::ProstCodec::default(),
                    )
                    .await
                    .unwrap()
                    .into_inner()
                    .boxed()
            }
            Self::Edge(session) => {
                let mut command = session
                    .command(kind, value.encode_to_vec(), Duration::from_secs(90), true)
                    .unwrap();
                Box::pin(async_stream::try_stream! {
                    while let Some(bytes) = command.next(&CancellationToken::new()).await.map_err(|e| Status::unknown(e.to_string()))? {
                        yield R::decode(bytes.as_slice()).map_err(|e| Status::internal(e.to_string()))?;
                    }
                })
            }
        }
    }
    async fn unary<Q, R>(&self, kind: Kind, path: &'static str, value: Q) -> R
    where
        Q: Message + Default + Send + Sync + 'static,
        R: Message + Default + Send + Sync + 'static,
    {
        match self {
            Self::Direct(channel) => {
                let mut client = tonic::client::Grpc::new(channel.clone());
                client.ready().await.unwrap();
                client
                    .unary(
                        sign(value, path),
                        axum::http::uri::PathAndQuery::from_static(path),
                        tonic_prost::ProstCodec::default(),
                    )
                    .await
                    .unwrap()
                    .into_inner()
            }
            Self::Edge(session) => {
                let mut command = session
                    .command(kind, value.encode_to_vec(), Duration::from_secs(30), false)
                    .unwrap();
                let bytes = command
                    .next(&CancellationToken::new())
                    .await
                    .unwrap()
                    .unwrap();
                assert!(
                    command
                        .next(&CancellationToken::new())
                        .await
                        .unwrap()
                        .is_none()
                );
                R::decode(bytes.as_slice()).unwrap()
            }
        }
    }
}

async fn docker_output(args: &[&str]) -> std::process::Output {
    tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new("docker")
            .args(args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn docker(args: &[&str]) -> String {
    let output = docker_output(args).await;
    assert!(
        output.status.success(),
        "docker {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

struct Fixture {
    root: PathBuf,
    stop: CancellationToken,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.cancel();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn auth(password: &str) -> String {
    STANDARD.encode(serde_json::json!({"serveraddress":"registry:5000","username":"fixture","password":password}).to_string())
}
async fn image_result(
    mut stream: BoxStream<'static, Result<ImageBuildResponse, Status>>,
    success: bool,
) -> Vec<String> {
    tokio::time::timeout(Duration::from_secs(90), async {
        let mut completed = 0;
        let mut errors = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.unwrap();
            assert!(
                !format!("{item:?}").contains(SECRET),
                "Build secret leaked into progress"
            );
            if let Some(error) = item.error_message.filter(|v| !v.is_empty()) {
                assert!(!success, "unexpected build failure: {error}");
                errors.push(error);
            }
            if item.status.as_deref() == Some("completed") {
                completed += 1;
            }
        }
        assert_eq!(completed, usize::from(success));
        assert_eq!(!errors.is_empty(), !success);
        errors
    })
    .await
    .unwrap()
}
async fn current_session(
    registry: &EdgeRegistry,
    target: &EdgeTarget,
    previous: Option<Uuid>,
) -> Arc<EdgeSession> {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(session) = registry.get(target)
                && Some(session.id) != previous
            {
                // Core registers its session before Agent persists SessionAccepted.
                // A completed command proves that Agent finished accepting it.
                let mut health = session
                    .command(
                        Kind::PlatformCheckHealth,
                        Vec::new(),
                        Duration::from_secs(5),
                        false,
                    )
                    .unwrap();
                health
                    .next(&CancellationToken::new())
                    .await
                    .unwrap()
                    .unwrap();
                return session;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Agent must enroll/reconnect to the real Core intake")
}

async fn start_agent(values: &HashMap<&str, String>) -> (tokio::process::Child, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("http://127.0.0.1:{port}");
    let mut child = tokio::process::Command::new("/app/citadel-agent")
        .envs(values)
        .env("CITADEL_AGENT_PORT", port.to_string())
        .kill_on_drop(true)
        .spawn()
        .expect("packaged Agent executable");
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            assert!(
                child.try_wait().unwrap().is_none(),
                "Agent exited during startup"
            );
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Agent listener startup");
    (child, address)
}

#[tokio::test]
#[ignore = "requires test-agent-compatibility.sh; never run on a shared Docker daemon"]
async fn direct_and_edge_build_pools_execute_real_docker_operations() {
    assert!(
        docker(&["info", "--format", "{{.Name}}"])
            .await
            .starts_with("citadel-agent-compat-"),
        "refusing a non-fixture Docker daemon"
    );
    let docker_host = std::env::var("DOCKER_HOST").unwrap();
    let database = std::env::var("CITADEL_AGENT_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&database)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&database)
        .await
        .unwrap();
    let fixture = Fixture {
        root: std::env::temp_dir().join(format!("compatibility-{}", Uuid::now_v7())),
        stop: CancellationToken::new(),
    };
    std::fs::create_dir(&fixture.root).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let core_url = format!("http://{}", listener.local_addr().unwrap());
    let registry = EdgeRegistry::default();
    let store = PostgresEdgeStore::new(pool.clone());
    let intake = EdgeIntake::new(store.clone(), registry.clone());
    let stop = fixture.stop.clone();
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

    for profile in ["direct", "edge-agent", "edge-build-agent"] {
        let id = Uuid::now_v7();
        let target = if profile == "edge-build-agent" {
            EdgeTarget::build_pool(id)
        } else {
            EdgeTarget::platform(id)
        };
        let token = if profile != "direct" {
            if profile == "edge-build-agent" {
                sqlx::query("INSERT INTO buildagentpools(id,name,normalizedname,createdbyactorid,provider,providerspec) VALUES($1,$2,$2,$3,'SelfManagedVm','{\"$type\":\"SelfManagedVm\",\"ConnectionMode\":\"EdgeAgent\"}')")
                    .bind(id).bind(id.to_string()).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
            } else {
                sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'EdgeAgent',0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Offline',0)")
                    .bind(id).execute(&pool).await.unwrap();
            }
            Some(
                store
                    .create_enrollment(&target, citadel_identity::SYSTEM_ACTOR_ID)
                    .await
                    .unwrap()
                    .1,
            )
        } else {
            None
        };
        let mut values = HashMap::from([
            ("DOCKER_HOST", docker_host.clone()),
            (
                "HUB_PUBLIC_KEY",
                AgentRequestSigner::from_bytes(&[42; 32]).public_key_base64(),
            ),
        ]);
        if let Some(token) = token {
            values.extend([
                ("CITADEL_AGENT_MODE", "edge".into()),
                ("CITADEL_CORE_URL", core_url.clone()),
                ("CITADEL_EDGE_AGENT_PROFILE", profile.into()),
                ("CITADEL_EDGE_ENROLLMENT_TOKEN", token),
                (
                    "CITADEL_EDGE_AGENT_KEY_PATH",
                    fixture
                        .root
                        .join(format!("{profile}.key"))
                        .display()
                        .to_string(),
                ),
                (
                    "CITADEL_EDGE_IDENTITY_PATH",
                    fixture
                        .root
                        .join(format!("{profile}.identity"))
                        .display()
                        .to_string(),
                ),
            ]);
        }
        let (mut running, address) = start_agent(&values).await;
        let wire = if profile == "direct" {
            let client = AgentClient::connect(
                &address,
                AgentRequestSigner::from_bytes(&[42; 32]),
                Duration::from_secs(30),
                true,
            )
            .await
            .unwrap();
            client.handshake(&CancellationToken::new()).await.unwrap();
            assert!(
                client
                    .check_build_host(&CancellationToken::new())
                    .await
                    .unwrap()
                    .available
            );
            Wire::Direct(
                Channel::from_shared(address)
                    .unwrap()
                    .connect()
                    .await
                    .unwrap(),
            )
        } else {
            let first = current_session(&registry, &target, None).await;
            let key = std::fs::read(values["CITADEL_EDGE_AGENT_KEY_PATH"].as_str()).unwrap();
            // Restart without the enrollment credential; the durable Build Pool identity must survive.
            running.kill().await.unwrap();
            values.remove("CITADEL_EDGE_ENROLLMENT_TOKEN");
            (running, _) = start_agent(&values).await;
            let next = current_session(&registry, &target, Some(first.id)).await;
            assert_eq!(first.agent_id, next.agent_id);
            assert_eq!(
                key,
                std::fs::read(values["CITADEL_EDGE_AGENT_KEY_PATH"].as_str()).unwrap()
            );
            Wire::Edge(next)
        };
        let ready: CheckBuildHostResponse = wire
            .unary(
                Kind::ImageCheckBuildHost,
                "/citadel.images.v1.ImageService/CheckBuildHost",
                (),
            )
            .await;
        assert!(ready.available);
        let process =
            std::fs::read_to_string(format!("/proc/{}/status", running.id().unwrap())).unwrap();
        let resident = process
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .unwrap();
        println!("{profile}: idle Agent {resident}");
        let image = format!("registry:5000/compat-{profile}:{}", id.simple());
        let context = fixture.root.join(profile);
        std::fs::create_dir(&context).unwrap();
        std::fs::write(context.join("Dockerfile"), "FROM compatibility-base:local\nRUN --mount=type=secret,id=fixture,required=true test -s /run/secrets/fixture && echo built > /marker\nCMD [\"sleep\",\"300\"]\n").unwrap();
        let archive = tokio::process::Command::new("tar")
            .args(["-C", context.to_str().unwrap(), "-cf", "-", "."])
            .output()
            .await
            .unwrap();
        assert!(archive.status.success());
        let build = BuildImageRequest {
            context_archive: archive.stdout,
            dockerfile_archive_path: Some("Dockerfile".into()),
            tags: vec![image.clone()],
            timeout_seconds: 60,
            build_secrets: vec![BuildImageSecret {
                id: "fixture".into(),
                value: SECRET.into(),
            }],
            ..Default::default()
        };
        image_result(
            wire.stream(
                Kind::ImageBuildStream,
                "/citadel.images.v1.ImageService/Build",
                build,
            )
            .await,
            true,
        )
        .await;
        assert_eq!(
            docker(&[
                "run",
                "--rm",
                &image,
                "sh",
                "-c",
                "test ! -e /run/secrets/fixture && cat /marker"
            ])
            .await,
            "built"
        );
        assert!(!docker(&["image", "inspect", &image]).await.contains(SECRET));
        for password in ["wrong-password", PASSWORD] {
            image_result(
                wire.stream(
                    Kind::ImagePushStream,
                    "/citadel.images.v1.ImageService/Push",
                    PushImageRequest {
                        image_reference: image.clone(),
                        registry_auth: Some(auth(password)),
                    },
                )
                .await,
                password == PASSWORD,
            )
            .await;
        }
        docker(&["image", "rm", &image]).await;
        let mut pull: BoxStream<'_, Result<PullImageResponse, Status>> = wire
            .stream(
                Kind::ImagePullStream,
                "/citadel.images.v1.ImageService/Pull",
                PullImageRequest {
                    from_image: image.clone(),
                    auth: Some(auth(PASSWORD)),
                    ..Default::default()
                },
            )
            .await;
        while let Some(item) = pull.next().await {
            assert!(item.unwrap().error_message.is_none());
        }
        assert!(
            !docker(&[
                "image",
                "inspect",
                &image,
                "--format",
                "{{json .RepoDigests}}"
            ])
            .await
            .is_empty()
        );

        for swarm in [false, true] {
            let project = format!(
                "compat-{}-{}",
                if swarm { "swarm" } else { "compose" },
                id.simple()
            );
            let request = StackApplyRequest {
                stack_name: project.clone(),
                project_name: Some(project.clone()),
                registry_auth: Some(STANDARD.encode(format!("fixture:{PASSWORD}"))),
                registry_host: Some("registry:5000".into()),
                orchestration_mode: i32::from(swarm),
                compose_file_content: Some(format!(
                    "services:\n  fixture:\n    image: {image}\n    command: ['sleep', '300']\n"
                )),
                ..Default::default()
            };
            let mut stack: BoxStream<'_, Result<StackApplyResponse, Status>> = wire
                .stream(
                    Kind::StackApplyStream,
                    "/citadel.stacks.v1.StackService/Apply",
                    request,
                )
                .await;
            let mut completed = 0;
            let mut final_statuses = Vec::new();
            while let Some(item) = stack.next().await {
                let item = item.unwrap();
                if item.r#type == 4 {
                    assert_eq!(item.exit_code, Some(0), "{profile}: {item:?}");
                    completed += 1;
                    if let Some(status) = item.stack_status {
                        final_statuses.push(status);
                    }
                }
            }
            assert!(completed > 0, "{profile}: stack stream never completed");
            assert_eq!(final_statuses, ["Healthy"], "{profile}");
            if swarm {
                tokio::time::timeout(Duration::from_secs(45), async {
                    loop {
                        if docker(&["stack", "services", &project, "--format", "{{.Replicas}}"])
                            .await
                            == "1/1"
                        {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                })
                .await
                .expect("Swarm stack must run its authenticated image");
                swarm::remove_stack(&project).await;
            } else {
                let ids = docker(&[
                    "ps",
                    "-aq",
                    "--filter",
                    &format!("label=com.docker.compose.project={project}"),
                ])
                .await;
                assert!(!ids.is_empty());
                for id in ids.lines() {
                    docker(&["rm", "-f", id]).await;
                }
                docker(&["network", "rm", &format!("{project}_default")]).await;
            }
        }
        let nodes: ListSwarmNodesResponse = wire
            .unary(
                Kind::SwarmNodeList,
                "/citadel.swarm.v1.SwarmService/ListNodes",
                ListSwarmNodesRequest::default(),
            )
            .await;
        assert_eq!(nodes.nodes.len(), 2);
        let created: SwarmResourceCreateResponse = wire
            .unary(
                Kind::SwarmSecretCreate,
                "/citadel.swarm.v1.SwarmService/CreateSecret",
                CreateSwarmSecretRequest {
                    name: format!("compat-{}", id.simple()),
                    data: b"fixture".to_vec(),
                    labels: HashMap::from([("fixture".into(), profile.into())]),
                },
            )
            .await;
        let _: () = wire
            .unary(
                Kind::SwarmSecretDelete,
                "/citadel.swarm.v1.SwarmService/DeleteSecret",
                DeleteSwarmSecretRequest {
                    secret_id: created.resource_id,
                },
            )
            .await;

        std::fs::write(
            context.join("Dockerfile"),
            "FROM compatibility-base:local\nRUN echo compatibility-waiting; sleep 120\n",
        )
        .unwrap();
        let slow = BuildImageRequest {
            context_directory: context.display().to_string(),
            dockerfile_path: context.join("Dockerfile").display().to_string(),
            timeout_seconds: 1,
            ..Default::default()
        };
        let timeout_errors = image_result(
            wire.stream(
                Kind::ImageBuildStream,
                "/citadel.images.v1.ImageService/Build",
                slow.clone(),
            )
            .await,
            false,
        )
        .await;
        assert!(
            timeout_errors
                .iter()
                .any(|error| error.contains("timed out")),
            "{timeout_errors:?}"
        );
        let mut slow = slow;
        slow.timeout_seconds = 60;
        let mut cancelled: BoxStream<'_, Result<ImageBuildResponse, Status>> = wire
            .stream(
                Kind::ImageBuildStream,
                "/citadel.images.v1.ImageService/Build",
                slow,
            )
            .await;
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let item = cancelled
                    .next()
                    .await
                    .expect("build stream ended before running")
                    .unwrap();
                assert!(item.error_message.is_none(), "{item:?}");
                if item
                    .stream
                    .as_deref()
                    .is_some_and(|line| line.contains("compatibility-waiting"))
                {
                    break;
                }
            }
        })
        .await
        .expect("build must start before cancellation");
        drop(cancelled);
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if !std::fs::read_dir("/tmp").unwrap().any(|v| {
                    v.unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with("citadel-build-config-")
                }) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("cancelled build must release its private credentials and process");
        running.kill().await.unwrap();
        if profile != "direct" {
            // Release this profile's daemon binding before enrolling the next fixture.
            store.revoke(&target).await.unwrap();
        }
        println!(
            "{profile}: build, secret, auth push/pull, stacks, Swarm mutation, timeout and cancellation passed"
        );
    }
    swarm::check(&pool, &store, &registry, &core_url, &fixture.root).await;
    fixture.stop.cancel();
    core.await.unwrap();
    pool.close().await;
}
