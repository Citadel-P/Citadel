use super::super::{AgentEnvelope, outgoing::QueuedEnvelope};
use super::*;
use axum::{Json, Router, body::Body, response::IntoResponse};
use citadel_contracts::citadel::{
    containers::v1::*, shared_models::v1::VolumeResponse, volumes::v1::CreateVolumeRequest,
};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::mpsc::Receiver;

struct Harness {
    docker: DockerClient,
    identity: Identity,
    calls: Arc<AtomicUsize>,
    streams: Arc<AtomicUsize>,
    created: Arc<Mutex<Vec<serde_json::Value>>>,
    stop: CancellationToken,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
struct LiveStream(Arc<AtomicUsize>);
impl Drop for LiveStream {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl Harness {
    async fn new() -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let streams = Arc::new(AtomicUsize::new(0));
        let created = Arc::new(Mutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(5),
            "/host",
        )
        .unwrap();
        let stop = CancellationToken::new();
        let router = Router::new().fallback({
            let calls = calls.clone(); let streams = streams.clone(); let created = created.clone();
            move |request: axum::extract::Request| {
                let calls = calls.clone(); let streams = streams.clone(); let created = created.clone();
                async move {
                    use serde_json::json;
                    calls.fetch_add(1, Ordering::SeqCst);
                    match request.uri().path() {
                        "/version" => Json(json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                        "/_ping" => "OK".into_response(),
                        "/v1.49/volumes/create" => {
                            let body = axum::body::to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
                            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                            created.lock().unwrap().push(body.clone());
                            Json(json!({"Name":body["Name"],"Driver":"local","Mountpoint":"/volume","Labels":body["Labels"],"Options":{},"Scope":"local"})).into_response()
                        }
                        "/v1.49/containers/failure/json" => (axum::http::StatusCode::NOT_FOUND, Json(json!({"message":"Container fixture is missing"}))).into_response(),
                        "/v1.49/containers/pending/json" => {
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            Json(json!({})).into_response()
                        }
                        "/v1.49/events" => {
                            streams.fetch_add(1, Ordering::SeqCst);
                            let guard = LiveStream(streams);
                            Body::from_stream(async_stream::stream! {
                                let _guard = guard;
                                yield Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"{\"Type\":\"service\",\"Action\":\"create\",\"Actor\":{\"ID\":\"service\",\"Attributes\":{}},\"scope\":\"swarm\"}\n"));
                                std::future::pending::<()>().await;
                            }).into_response()
                        }
                        _ => (axum::http::StatusCode::BAD_REQUEST, Json(json!({"message":"Unexpected fixture route"}))).into_response(),
                    }
                }
            }
        });
        let stopped = stop.clone();
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(stopped.cancelled_owned())
                .await
                .unwrap();
        });
        let id = Uuid::now_v7();
        Self {
            docker,
            identity: Identity {
                platform_id: id,
                resource_id: id,
                resource_type: "Platform".into(),
                agent_id: Uuid::now_v7(),
                agent_fingerprint: String::new(),
                enrollment_token_fingerprint: None,
            },
            calls,
            streams,
            created,
            stop,
        }
    }
    fn commands(&self) -> (Commands, Receiver<QueuedEnvelope>) {
        let (outgoing, receiver) = Outgoing::channel();
        (
            Commands::new(
                self.docker.clone(),
                None,
                self.identity.clone(),
                EdgeProfile::Ordinary,
                Uuid::now_v7().to_string(),
                outgoing,
            ),
            receiver,
        )
    }
    fn command(&self, kind: EdgeCommandKind, payload: impl Message) -> EdgeCommand {
        EdgeCommand {
            command_id: Uuid::now_v7().to_string(),
            platform_id: self.identity.platform_id.to_string(),
            resource_type: 0,
            resource_id: self.identity.resource_id.to_string(),
            kind: kind as i32,
            payload: payload.encode_to_vec(),
            payload_schema_version: 1,
            ..Default::default()
        }
    }
}
async fn start(commands: &mut Commands, command: EdgeCommand) {
    commands
        .start(&command.command_id.clone(), command)
        .await
        .unwrap();
}
async fn finish(commands: &mut Commands) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while commands.running() {
            commands.next().await.unwrap();
        }
    })
    .await
    .expect("command must finish");
}
fn take(receiver: &mut Receiver<QueuedEnvelope>) -> Vec<AgentEnvelope> {
    let mut values = vec![];
    while let Ok(value) = receiver.try_recv() {
        values.push(value.message);
    }
    values
}
fn failed(message: &AgentEnvelope, code: &str) {
    assert!(
        matches!(&message.body, Some(AgentBody::CommandFailed(value)) if value.code == code),
        "{message:?}"
    );
}

#[tokio::test]
async fn every_generated_command_reaches_its_protobuf_decoder() {
    let h = Harness::new().await;
    let (outgoing, _receiver) = Outgoing::channel();
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../infrastructure/contracts/proto/edge_agent_service.proto"
    ));
    let enumeration = source
        .split("enum EdgeCommandKind {")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    let mut count = 0;
    for line in enumeration.lines().filter(|line| line.contains('=')) {
        let (name, number) = line.trim().trim_end_matches(';').split_once('=').unwrap();
        let kind = EdgeCommandKind::try_from(number.trim().parse::<i32>().unwrap()).unwrap();
        assert_eq!(kind.as_str_name(), name.trim());
        let error = dispatch::execute(
            Runtime::new(h.docker.clone(), CancellationToken::new(), None),
            kind,
            &[0xff],
            Box::pin(futures_util::stream::empty()),
            Output {
                outgoing: outgoing.clone(),
                session: "session".into(),
                command: "command".into(),
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument, "{kind:?}");
        if kind != EdgeCommandKind::Unspecified {
            assert_eq!(
                error.message(),
                "Invalid Edge command protobuf payload",
                "{kind:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 68);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unary_success_and_failure_emit_exactly_one_terminal_and_preserve_maps() {
    let h = Harness::new().await;
    let (mut commands, mut receiver) = h.commands();
    let good = h.command(
        EdgeCommandKind::VolumeCreate,
        CreateVolumeRequest {
            name: "fixture".into(),
            labels: HashMap::from([("z".into(), "last".into()), ("a".into(), "first".into())]),
            ..Default::default()
        },
    );
    let id = good.command_id.clone();
    start(&mut commands, good).await;
    finish(&mut commands).await;
    let messages = take(&mut receiver);
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().all(|value| {
        value.command_id == id
            && Uuid::parse_str(&value.envelope_id)
                .unwrap()
                .get_version_num()
                == 7
    }));
    let Some(AgentBody::CommandOutput(value)) = &messages[0].body else {
        panic!("output")
    };
    let volume = VolumeResponse::decode(value.payload.as_slice()).unwrap();
    assert_eq!(volume.name, "fixture");
    assert_eq!(h.created.lock().unwrap()[0]["Labels"]["a"], "first");
    assert!(matches!(&messages[1].body,Some(AgentBody::CommandCompleted(v)) if v.status_code == 0));
    start(
        &mut commands,
        h.command(
            EdgeCommandKind::ContainerInspect,
            InspectContainerRequest {
                container_id: "failure".into(),
            },
        ),
    )
    .await;
    finish(&mut commands).await;
    let messages = take(&mut receiver);
    assert_eq!(messages.len(), 1);
    failed(&messages[0], "not_found");
    assert!(
        matches!(&messages[0].body,Some(AgentBody::CommandFailed(v)) if v.message.contains("fixture is missing"))
    );
}

#[tokio::test]
async fn limits_duplicates_bad_targets_schemas_and_swarm_fail_before_execution() {
    let h = Harness::new().await;
    let (mut commands, mut receiver) = h.commands();
    let first = h.command(EdgeCommandKind::PlatformCheckHealth, ());
    for _ in 0..16 {
        start(
            &mut commands,
            h.command(EdgeCommandKind::PlatformCheckHealth, ()),
        )
        .await;
    }
    start(&mut commands, first.clone()).await;
    failed(&take(&mut receiver)[0], "resource_exhausted");
    assert_eq!(commands.active.len(), 16);
    finish(&mut commands).await;
    take(&mut receiver);
    start(&mut commands, first.clone()).await;
    assert!(
        commands
            .start(&first.command_id, first.clone())
            .await
            .is_err()
    );
    assert_eq!(commands.active.len(), 1);
    commands.cancel(&first.command_id);
    finish(&mut commands).await;
    let messages = take(&mut receiver);
    assert_eq!(messages.len(), 1);
    failed(&messages[0], "cancelled");
    for (mut bad, code) in [
        (
            EdgeCommand {
                payload_schema_version: 2,
                ..first.clone()
            },
            "invalid_argument",
        ),
        (
            EdgeCommand {
                kind: 9999,
                ..first.clone()
            },
            "invalid_argument",
        ),
        (
            EdgeCommand {
                resource_id: Uuid::now_v7().to_string(),
                ..first.clone()
            },
            "permission_denied",
        ),
        (
            EdgeCommand {
                payload: vec![0; MAX_ENVELOPE + 1],
                ..first.clone()
            },
            "resource_exhausted",
        ),
        (
            EdgeCommand {
                node_id: "another-node".into(),
                ..first.clone()
            },
            "permission_denied",
        ),
    ] {
        bad.command_id = Uuid::now_v7().to_string();
        start(&mut commands, bad).await;
        let messages = take(&mut receiver);
        assert_eq!(messages.len(), 1);
        failed(&messages[0], code);
        assert!(!commands.running());
    }
    let before = h.calls.load(Ordering::SeqCst);
    commands.profile = EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
        bootstrap_file: "unused".into(),
        platform_id: h.identity.platform_id.to_string(),
        cluster_id: "cluster".into(),
        node_id: "node".into(),
        node_hostname: "worker".into(),
        service_id: "service".into(),
        task_id: "task".into(),
    });
    start(&mut commands, first).await;
    failed(&take(&mut receiver)[0], "permission_denied");
    assert_eq!(h.calls.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn deadline_cancel_input_overflow_and_session_drop_release_owned_work() {
    let h = Harness::new().await;
    let (mut commands, mut receiver) = h.commands();
    let mut request = h.command(
        EdgeCommandKind::ContainerInspect,
        InspectContainerRequest {
            container_id: "pending".into(),
        },
    );
    request.timeout_ms = 20;
    start(&mut commands, request).await;
    finish(&mut commands).await;
    failed(&take(&mut receiver)[0], "deadline_exceeded");
    let exec = h.command(
        EdgeCommandKind::ContainerExec,
        ExecClientMessage {
            msg: Some(exec_client_message::Msg::Open(ExecOpen {
                container_id: "fixture".into(),
                cmd: vec!["sh".into()],
                tty: true,
            })),
        },
    );
    let id = exec.command_id.clone();
    start(&mut commands, exec).await;
    let payload = ExecClientMessage {
        msg: Some(exec_client_message::Msg::Stdin(ExecStdin { data: vec![1] })),
    }
    .encode_to_vec();
    for _ in 0..=INPUT_CAPACITY {
        commands.input(
            &id,
            StreamInput {
                payload: payload.clone(),
            },
        );
    }
    assert!(
        commands.active[&Uuid::parse_str(&id).unwrap()]
            .cancel
            .is_cancelled()
    );
    finish(&mut commands).await;
    failed(&take(&mut receiver)[0], "cancelled");
    assert_eq!(commands.budget.available_permits(), INPUT_BYTES);
    let events = h.command(EdgeCommandKind::PlatformDaemonEventsStream, ());
    let id = events.command_id.clone();
    start(&mut commands, events).await;
    tokio::select! {
        result = commands.next() => panic!("stream finished early: {result:?}"),
        value = receiver.recv() => assert!(matches!(value.unwrap().message.body,Some(AgentBody::CommandOutput(_)))),
        () = tokio::time::sleep(Duration::from_secs(3)) => panic!("no event output"),
    }
    assert_eq!(h.streams.load(Ordering::SeqCst), 1);
    commands.cancel(&id);
    finish(&mut commands).await;
    let messages = take(&mut receiver);
    assert_eq!(messages.len(), 1);
    failed(&messages[0], "cancelled");
    start(
        &mut commands,
        h.command(EdgeCommandKind::PlatformDaemonEventsStream, ()),
    )
    .await;
    tokio::select! {
        result = commands.next() => panic!("stream finished early: {result:?}"),
        value = receiver.recv() => assert!(value.is_some()),
    }
    drop(commands);
    tokio::time::timeout(Duration::from_secs(3), async {
        while h.streams.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("session drop closes Docker streams");
    assert!(receiver.recv().await.is_none());
}

#[tokio::test]
async fn outgoing_queue_has_a_shared_byte_budget_and_rejects_oversized_messages() {
    let (outgoing, mut receiver) = Outgoing::channel();
    let output = Output {
        outgoing: outgoing.clone(),
        session: "session".into(),
        command: "command".into(),
    };
    assert_eq!(
        output
            .message(ExecStdin {
                data: vec![0; MAX_ENVELOPE]
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::ResourceExhausted
    );
    let message = || {
        super::super::envelope(
            "session",
            "command",
            AgentBody::CommandOutput(CommandOutput {
                payload: vec![0; MAX_ENVELOPE - 1024],
            }),
        )
    };
    outgoing.send(message()).await.unwrap();
    outgoing.send(message()).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(30), outgoing.send(message()))
            .await
            .is_err()
    );
    drop(receiver.recv().await.unwrap());
    tokio::time::timeout(Duration::from_secs(1), outgoing.send(message()))
        .await
        .unwrap()
        .unwrap();
}

async fn request(socket: &mut tokio::net::TcpStream) -> (String, Vec<u8>) {
    use tokio::io::AsyncReadExt;
    let mut headers = Vec::new();
    loop {
        headers.push(socket.read_u8().await.unwrap());
        if headers.ends_with(b"\r\n\r\n") {
            break;
        }
        assert!(headers.len() < 16384);
    }
    let headers = String::from_utf8(headers).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.unwrap();
    (headers, body)
}
async fn respond(socket: &mut tokio::net::TcpStream, body: &str) {
    use tokio::io::AsyncWriteExt;
    socket
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}
#[tokio::test]
async fn interactive_and_binary_exec_preserve_input_resize_output_bytes_and_exit() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for interactive in [true, false] {
        let h = Harness::new().await;
        let (mut commands, mut receiver) = h.commands();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        commands.docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(2),
            "/host",
        )
        .unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(request(&mut socket).await.0.contains("/version"));
            respond(
                &mut socket,
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            )
            .await;
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, body) = request(&mut socket).await;
            assert!(headers.contains("/containers/fixture/exec"));
            let config: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(config["Tty"], interactive);
            respond(&mut socket, r#"{"Id":"exec-1"}"#).await;
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(request(&mut stream).await.0.contains("/exec/exec-1/start"));
            stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n").await.unwrap();
            if interactive {
                let mut input = [0; 3];
                stream.read_exact(&mut input).await.unwrap();
                assert_eq!(&input, b"ls\n");
                let (mut resize, _) = listener.accept().await.unwrap();
                assert!(
                    request(&mut resize)
                        .await
                        .0
                        .contains("/exec/exec-1/resize?h=30&w=100")
                );
                respond(&mut resize, "").await;
            } else {
                stream.write_all(&[2, 0, 0, 0, 0, 0, 0, 3]).await.unwrap();
            }
            stream.write_all(&[0, 255, 42]).await.unwrap();
            drop(stream);
            let (mut inspect, _) = listener.accept().await.unwrap();
            assert!(request(&mut inspect).await.0.contains("/exec/exec-1/json"));
            respond(&mut inspect, r#"{"Running":false,"ExitCode":7}"#).await;
        });
        let command = if interactive {
            h.command(
                EdgeCommandKind::ContainerExec,
                ExecClientMessage {
                    msg: Some(exec_client_message::Msg::Open(ExecOpen {
                        container_id: "fixture".into(),
                        cmd: vec!["sh".into()],
                        tty: true,
                    })),
                },
            )
        } else {
            h.command(
                EdgeCommandKind::ContainerExecBinary,
                ExecBinaryRequest {
                    container_id: "fixture".into(),
                    cmd: vec!["cat".into(), "/data/file".into()],
                    ..Default::default()
                },
            )
        };
        let id = command.command_id.clone();
        start(&mut commands, command).await;
        if interactive {
            for message in [
                exec_client_message::Msg::Stdin(ExecStdin {
                    data: b"ls\n".to_vec(),
                }),
                exec_client_message::Msg::Resize(ExecResize {
                    rows: 30,
                    cols: 100,
                }),
            ] {
                commands.input(
                    &id,
                    StreamInput {
                        payload: ExecClientMessage { msg: Some(message) }.encode_to_vec(),
                    },
                );
            }
        }
        finish(&mut commands).await;
        let messages = take(&mut receiver);
        let mut bytes = vec![];
        let mut exits = vec![];
        let mut completed = 0;
        for message in messages {
            match message.body.unwrap() {
                AgentBody::CommandOutput(output) => {
                    match ExecServerMessage::decode(output.payload.as_slice())
                        .unwrap()
                        .msg
                        .unwrap()
                    {
                        exec_server_message::Msg::Output(value) => {
                            assert_eq!(value.stream, i32::from(!interactive));
                            bytes.extend(value.data);
                        }
                        exec_server_message::Msg::Exit(value) => exits.push(value.exit_code),
                        exec_server_message::Msg::Error(value) => panic!("exec error: {value:?}"),
                    }
                }
                AgentBody::CommandCompleted(value) => {
                    assert_eq!(value.status_code, 0);
                    completed += 1;
                }
                other => panic!("unexpected exec result: {other:?}"),
            }
        }
        assert_eq!(bytes, [0, 255, 42]);
        assert_eq!(exits, [7]);
        assert_eq!(completed, 1);
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn swarm_denials_never_reach_docker_and_matching_node_health_is_allowed() {
    let h = Harness::new().await;
    let (mut commands, mut receiver) = h.commands();
    commands.profile = EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
        bootstrap_file: "unused".into(),
        platform_id: h.identity.platform_id.to_string(),
        cluster_id: "cluster".into(),
        node_id: "node".into(),
        node_hostname: "worker".into(),
        service_id: "service".into(),
        task_id: "task".into(),
    });
    let mut helper = super::super::swarm_policy::volume_request(h.identity.platform_id);
    helper.privileged = Some(true);
    for mut command in [
        h.command(EdgeCommandKind::ImageBuildStream, ()),
        h.command(EdgeCommandKind::ContainerCreate, helper),
        h.command(
            EdgeCommandKind::ContainerExecBinary,
            ExecBinaryRequest {
                container_id: "fake-label".into(),
                cmd: vec!["sh".into()],
                ..Default::default()
            },
        ),
        h.command(
            EdgeCommandKind::VolumeDelete,
            citadel_contracts::citadel::volumes::v1::RemoveVolumeRequest {
                names: vec!["volume".into()],
                force: true,
            },
        ),
    ] {
        command.node_id = "node".into();
        start(&mut commands, command).await;
        finish(&mut commands).await;
        let values = take(&mut receiver);
        assert_eq!(values.len(), 1);
        failed(&values[0], "permission_denied");
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let mut command = h.command(EdgeCommandKind::PlatformCheckHealth, ());
    command.node_id = "node".into();
    start(&mut commands, command).await;
    finish(&mut commands).await;
    let values = take(&mut receiver);
    assert_eq!(values.len(), 2);
    assert!(matches!(
        values[1].body,
        Some(AgentBody::CommandCompleted(_))
    ));
}

#[tokio::test]
#[ignore = "requires Docker and CITADEL_AGENT_HELPER_TEST_IMAGE built from the agent-runtime-base target"]
async fn swarm_volume_browsing_runs_packaged_rust_helper_and_cleans_up() {
    use citadel_contracts::citadel::volumes::v1::{CreateVolumeRequest, RemoveVolumeRequest};
    let image = std::env::var("CITADEL_AGENT_HELPER_TEST_IMAGE").unwrap();
    let h = Harness::new().await;
    let (mut commands, mut receiver) = h.commands();
    commands.docker = DockerClient::with_endpoint(
        std::env::var("DOCKER_HOST")
            .unwrap_or_else(|_| "unix:///var/run/docker.sock".into())
            .parse()
            .unwrap(),
        Duration::from_secs(10),
        "/host",
    )
    .unwrap();
    commands.profile = EdgeProfile::SwarmNode(crate::config::SwarmIdentity {
        bootstrap_file: "unused".into(),
        platform_id: h.identity.platform_id.to_string(),
        cluster_id: "fixture-cluster".into(),
        node_id: "fixture-node".into(),
        node_hostname: "fixture-host".into(),
        service_id: "fixture-service".into(),
        task_id: "fixture-task".into(),
    });
    let name = format!("citadel-volume-helper-{}", Uuid::now_v7().simple());
    let volume = format!("citadel-agent-volume-test-{}", Uuid::now_v7().simple());
    struct Cleanup {
        name: String,
        volume: String,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::process::Command::new("docker")
                .args(["rm", "-f", &self.name])
                .output();
            let _ = std::process::Command::new("docker")
                .args(["volume", "rm", &self.volume])
                .output();
        }
    }
    let _cleanup = Cleanup {
        name: name.clone(),
        volume: volume.clone(),
    };
    let run = |args: &[&str]| {
        let output = std::process::Command::new("docker")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&[
        "volume",
        "create",
        "--label",
        &format!("citadel.platform-id={}", h.identity.platform_id),
        "--label",
        &format!("citadel.backup.restoreRunId={}", Uuid::now_v7()),
        &volume,
    ]);
    run(&[
        "run",
        "--rm",
        "--network",
        "none",
        "-v",
        &format!("{volume}:/data"),
        "--entrypoint",
        "/bin/sh",
        &image,
        "-c",
        "printf '\\000\\377\\052' > /data/binary; ln -s /etc /data/outside",
    ]);
    let node_command = |kind, payload: Vec<u8>| {
        let mut command = h.command(kind, ());
        command.node_id = "fixture-node".into();
        command.payload = payload;
        command
    };
    let mut helper = super::super::swarm_policy::volume_request(h.identity.platform_id);
    helper.name = name.clone();
    helper.image_id = image;
    helper.mounts[0].source = Some(volume.clone());
    helper.memory_limit = Some(128 * 1024 * 1024);
    helper.memory_swap = helper.memory_limit;
    helper.pids_limit = Some(64);
    start(
        &mut commands,
        node_command(EdgeCommandKind::ContainerCreate, helper.encode_to_vec()),
    )
    .await;
    finish(&mut commands).await;
    let values = take(&mut receiver);
    let Some(AgentBody::CommandOutput(output)) = &values[0].body else {
        panic!("{values:?}")
    };
    let id = CreateContainerResponse::decode(output.payload.as_slice())
        .unwrap()
        .container_id;
    start(
        &mut commands,
        node_command(
            EdgeCommandKind::ContainerStart,
            ContainerIds {
                ids: vec![id.clone()],
            }
            .encode_to_vec(),
        ),
    )
    .await;
    finish(&mut commands).await;
    assert!(matches!(
        take(&mut receiver).last().unwrap().body,
        Some(AgentBody::CommandCompleted(_))
    ));
    for (operation, path) in [
        ("list", "/"),
        ("stream-file", "/binary"),
        ("inspect", "/outside"),
    ] {
        let mut cmd = vec![
            if operation == "stream-file" {
                super::super::swarm_policy::HELPER_BINARY
            } else {
                super::super::swarm_policy::COMPAT_HELPER_BINARY
            }
            .into(),
            "volume-helper".into(),
            operation.into(),
            "--root".into(),
            "/data".into(),
            "--path".into(),
            path.into(),
        ];
        if operation == "list" {
            cmd.extend(
                ["--max-entries", "1000", "--max-payload-bytes", "1048576"].map(str::to_owned),
            );
        }
        start(
            &mut commands,
            node_command(
                EdgeCommandKind::ContainerExecBinary,
                ExecBinaryRequest {
                    container_id: id.clone(),
                    cmd,
                    ..Default::default()
                }
                .encode_to_vec(),
            ),
        )
        .await;
        finish(&mut commands).await;
        let values = take(&mut receiver);
        let mut bytes = vec![];
        let mut exit = None;
        for value in &values {
            if let Some(AgentBody::CommandOutput(out)) = &value.body {
                match ExecServerMessage::decode(out.payload.as_slice())
                    .unwrap()
                    .msg
                    .unwrap()
                {
                    exec_server_message::Msg::Output(out) => {
                        assert_eq!(out.stream, 0);
                        bytes.extend(out.data);
                    }
                    exec_server_message::Msg::Exit(code) => exit = Some(code.exit_code),
                    other => panic!("{other:?}"),
                }
            }
        }
        assert_eq!(exit, Some(0), "{values:?}");
        assert!(matches!(
            values.last().unwrap().body,
            Some(AgentBody::CommandCompleted(_))
        ));
        match operation {
            "list" => {
                let listing: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(listing["Entries"].as_array().unwrap().len(), 2);
            }
            "stream-file" => assert_eq!(bytes, [0, 255, 42]),
            _ => {
                let inspected: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(inspected["ErrorCode"], "VolumePathIsSymlink");
            }
        }
    }
    // An owned restore volume remains protected while a helper mounts it.
    start(
        &mut commands,
        node_command(
            EdgeCommandKind::VolumeDelete,
            RemoveVolumeRequest {
                names: vec![volume.clone()],
                force: false,
            }
            .encode_to_vec(),
        ),
    )
    .await;
    finish(&mut commands).await;
    failed(&take(&mut receiver)[0], "permission_denied");
    start(
        &mut commands,
        node_command(
            EdgeCommandKind::ContainerDelete,
            DeleteContainerRequest {
                ids: vec![id],
                force: Some(true),
                ..Default::default()
            }
            .encode_to_vec(),
        ),
    )
    .await;
    finish(&mut commands).await;
    assert!(matches!(
        take(&mut receiver).last().unwrap().body,
        Some(AgentBody::CommandCompleted(_))
    ));
    let restore = format!("{}-restore", volume);
    let _restore_cleanup = Cleanup {
        name: String::new(),
        volume: restore.clone(),
    };
    let restore_request = CreateVolumeRequest {
        name: restore.clone(),
        driver: "local".into(),
        labels: HashMap::from([
            (
                "citadel.platform-id".into(),
                h.identity.platform_id.to_string(),
            ),
            (
                "citadel.backup.restoreRunId".into(),
                Uuid::now_v7().to_string(),
            ),
        ]),
        ..Default::default()
    };
    start(
        &mut commands,
        node_command(
            EdgeCommandKind::VolumeCreate,
            restore_request.encode_to_vec(),
        ),
    )
    .await;
    finish(&mut commands).await;
    assert!(matches!(
        take(&mut receiver).last().unwrap().body,
        Some(AgentBody::CommandCompleted(_))
    ));
    start(
        &mut commands,
        node_command(
            EdgeCommandKind::VolumeDelete,
            RemoveVolumeRequest {
                names: vec![restore],
                force: false,
            }
            .encode_to_vec(),
        ),
    )
    .await;
    finish(&mut commands).await;
    assert!(matches!(
        take(&mut receiver).last().unwrap().body,
        Some(AgentBody::CommandCompleted(_))
    ));
}
