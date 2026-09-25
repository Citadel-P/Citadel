use super::*;
use citadel_adapters::persistence::postgres::{
    activities::store::PostgresActivityStore, alerts::PostgresAlertRepository,
    automation::PostgresAutomationRepository, backups::PostgresBackupPersistence,
    builds::PostgresBuildRepository, deployments::PostgresDeploymentRepository,
    stacks::PostgresStackRepository, swarm_services::PostgresSwarmServiceRepository,
};
use citadel_identity::{AccessTokenClaims, SessionTokenCodec};
use citadel_primitives::ResourceType;
use citadel_server::{
    config::RealtimeConfig,
    metrics::Metrics,
    realtime::{IdentityRealtimeReader, RealtimeService},
    realtime_groups::{ApplicationGroupReader, Group, GroupReadPort},
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tokio_util::sync::CancellationToken;
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub(super) fn reader(f: &Fixture) -> ApplicationGroupReader {
    ApplicationGroupReader {
        git_repositories: Arc::new(
            citadel_adapters::persistence::postgres::git::repositories::PostgresGitRepositoryPersistence::new(
                f.pool.clone(),
            ),
        ),
        identity: f.lookup_state.platforms.identity.clone(),
        platforms: f.lookup_state.platforms.platforms.clone(),
        deployments: Arc::new(PostgresDeploymentRepository::new(f.pool.clone())),
        stacks: Arc::new(PostgresStackRepository::new(f.pool.clone())),
        services: Arc::new(PostgresSwarmServiceRepository::new(f.pool.clone())),
        automation: Arc::new(PostgresAutomationRepository::new(f.pool.clone())),
        builds: Arc::new(PostgresBuildRepository::new(f.pool.clone())),
        backups: Arc::new(PostgresBackupPersistence::new(f.pool.clone())),
        activities: Arc::new(citadel_activities::ActivityService::new(Arc::new(
            PostgresActivityStore::new(f.pool.clone()),
        ))),
        alerts: Arc::new(PostgresAlertRepository::new(f.pool.clone())),
        docker: f.lookup_state.platforms.clone(),
    }
}
fn token(p: &ActorPrincipal) -> String {
    JwtSessionTokenCodec::new(&[7; 32], "fixture".into(), "fixture".into())
        .unwrap()
        .encode_access(&AccessTokenClaims {
            subject_id: p.subject_id,
            actor_id: p.actor_id,
            principal_type: p.principal_type,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
            automation_run_id: None,
        })
        .unwrap()
}
async fn receive(socket: &mut Socket) -> Value {
    let frame = tokio::time::timeout(StdDuration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(frame.to_text().unwrap()).unwrap()
}

// Local counterpart to ExecSessionManagerTests and the browser's Join -> Start
// -> Resize -> typed-array stdin sequence. Uses a Docker socket fixture by
// default; optional environment variables target a disposable real container.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; optional CITADEL_TERMINAL_TEST_CONTAINER/SOCKET enable live Docker"]
async fn local_terminal_browser_sequence_streams_output_and_reconnects() {
    let (docker_id, docker_socket, fixture_server) = match (
        std::env::var("CITADEL_TERMINAL_TEST_CONTAINER").ok(),
        std::env::var("CITADEL_TERMINAL_TEST_SOCKET").ok(),
    ) {
        (Some(id), Some(socket)) => (id, PathBuf::from(socket), None),
        (None, None) => {
            let id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
            let path = std::env::temp_dir().join(format!("terminal-{}.sock", Uuid::now_v7()));
            let listener = UnixListener::bind(&path).unwrap();
            let target = id.clone();
            (
                id,
                path,
                Some(tokio::spawn(async move {
                    local_terminal_fixture(listener, &target).await;
                })),
            )
        }
        _ => panic!("Set both terminal fixture environment variables or neither"),
    };
    let f = fixture().await;
    let id: Uuid = sqlx::query_scalar("UPDATE containers SET dockernodeid=NULL,dockercontainerid=$2 WHERE platformid=$1 RETURNING id")
        .bind(f.platform_id).bind(&docker_id).fetch_one(&f.pool).await.unwrap();
    let mut reader = reader(&f);
    reader.docker.docker = DockerClient::new(&docker_socket, StdDuration::from_secs(5)).unwrap();
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(30),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&f.administrator)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    for (reference, shell) in [(id.to_string(), "bash"), (docker_id[..12].to_owned(), "sh")] {
        let group = format!("container-exec:{reference}:browser-session");
        let mut output = Vec::new();
        for (target, args) in [
            ("JoinGroup", json!([group])),
            (
                "StartExecProcess",
                json!([reference, "browser-session", shell]),
            ),
            ("ResizeExec", json!([reference, "browser-session", 100, 30])),
            (
                "SendExecInput",
                json!([
                    reference,
                    "browser-session",
                    "printf 'citadel-%s\\n' terminal-ok\n"
                        .bytes()
                        .enumerate()
                        .map(|(i, b)| (i.to_string(), json!(b)))
                        .collect::<serde_json::Map<_, _>>()
                ]),
            ),
        ] {
            socket.send(Message::Text(json!({"protocolVersion":1,"kind":"invoke","invocationId":target,"target":target,"arguments":args}).to_string().into())).await.unwrap();
            loop {
                let frame = receive(&mut socket).await;
                if frame["kind"] == "completion" {
                    assert_eq!(frame["invocationId"], target);
                    assert!(frame["error"].is_null(), "{target}: {frame}");
                    break;
                }
                append_terminal_output(&frame, &mut output);
            }
        }
        while !String::from_utf8_lossy(&output).contains("citadel-terminal-ok") {
            append_terminal_output(&receive(&mut socket).await, &mut output);
        }
        // Ignore pending output until Leave completes. The next iteration must
        // create a new session on the same shared realtime connection.
        socket.send(Message::Text(json!({"protocolVersion":1,"kind":"invoke","invocationId":"leave","target":"LeaveGroup","arguments":[group]}).to_string().into())).await.unwrap();
        loop {
            let frame = receive(&mut socket).await;
            if frame["kind"] == "completion" {
                assert!(frame["error"].is_null());
                break;
            }
        }
    }
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    if let Some(server) = fixture_server {
        tokio::time::timeout(StdDuration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        std::fs::remove_file(docker_socket).unwrap();
    }
    cleanup(f).await;
}

async fn local_terminal_fixture(listener: UnixListener, docker_id: &str) {
    for shell in ["/bin/bash", "/bin/sh"] {
        if shell == "/bin/bash" {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, _) = read_terminal_request(&mut socket).await;
            assert!(headers.starts_with("GET /version "));
            let body = r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#;
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
        let (mut create, _) = listener.accept().await.unwrap();
        let (headers, body) = read_terminal_request(&mut create).await;
        assert!(headers.starts_with(&format!("POST /v1.49/containers/{docker_id}/exec ")));
        assert_eq!(body["Cmd"], json!([shell]));
        let body = r#"{"Id":"exec-1"}"#;
        create
            .write_all(
                format!(
                    "HTTP/1.1 201 Created\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let (mut terminal, _) = listener.accept().await.unwrap();
        let (headers, _) = read_terminal_request(&mut terminal).await;
        assert!(headers.starts_with("POST /v1.49/exec/exec-1/start "));
        terminal
            .write_all(b"HTTP/1.1 101 UPGRADED\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n")
            .await
            .unwrap();
        let (mut resize, _) = listener.accept().await.unwrap();
        let (headers, _) = read_terminal_request(&mut resize).await;
        assert!(headers.starts_with("POST /v1.49/exec/exec-1/resize?h=30&w=100 "));
        resize
            .write_all(b"HTTP/1.1 409 Conflict\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut input = vec![0; "printf 'citadel-%s\\n' terminal-ok\n".len()];
        terminal.read_exact(&mut input).await.unwrap();
        assert_eq!(input, b"printf 'citadel-%s\\n' terminal-ok\n");
        terminal
            .write_all(b"citadel-terminal-ok\r\n")
            .await
            .unwrap();
        assert_eq!(terminal.read(&mut input).await.unwrap(), 0);
    }
}

async fn read_terminal_request(socket: &mut tokio::net::UnixStream) -> (String, Value) {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        headers.push(socket.read_u8().await.unwrap());
        assert!(headers.len() < 16384);
    }
    let headers = String::from_utf8(headers).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(length < 16384);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.unwrap();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (headers, body)
}

fn append_terminal_output(frame: &Value, output: &mut Vec<u8>) {
    assert_eq!(frame["target"], "SendContainerExec", "{frame}");
    output.extend(
        frame["arguments"][0]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap()),
    );
    assert!(output.len() < 65536);
}
async fn invoke(socket: &mut Socket, target: &str, group: &str) -> Value {
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"invoke","invocationId":"1","target":target,"arguments":[group]}).to_string().into())).await.unwrap();
    let completion = receive(socket).await;
    assert_eq!(completion["kind"], "completion");
    completion
}
async fn cleanup(f: Fixture) {
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_batch_shares_one_projection_but_rechecks_each_readers_authorization() {
    let f = fixture().await;
    let mut inventory = snapshot(f.platform_id);
    let template = inventory.containers[0].clone();
    inventory.containers = (0..6)
        .map(|i| {
            let mut container = template.clone();
            container.id = format!("batch-{i}");
            container.name = format!("batch-{i}");
            container
        })
        .collect();
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    f.docker_server.abort();
    let reader = reader(&f);
    let ids: Vec<_> = (0..5).map(|i| format!("batch-{i}")).collect();
    let hub = citadel_server::realtime::RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut first = hub.subscribe();
    let mut second = hub.subscribe();
    hub.publish_container_changes(f.platform_id, "update", &ids);
    let change = first.recv().await.unwrap();
    let shared = second.recv().await.unwrap();
    let daemon = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    let containers = Group::parse(&format!("containers:{}", f.platform_id)).unwrap();
    let initial = reader
        .read(&f.administrator, &daemon, Some(&change))
        .await
        .unwrap();
    assert_eq!(initial.rows.len(), 5);
    assert!(
        initial
            .rows
            .iter()
            .all(|row| row.rows.len() == 1 && row.rows[0]["containerId"] != "batch-5")
    );
    let original_state = initial.rows[0].rows[0]["state"].clone();
    sqlx::query("UPDATE containers SET state='Exited' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    // Another group on another socket reuses this invalidation's projection.
    // The next invalidation must get fresh rows, rather than a timed cache.
    let reused = reader
        .read(&f.administrator, &containers, Some(&shared))
        .await
        .unwrap();
    assert_eq!(reused.events.len(), 5);
    assert_eq!(
        reused.events[0].arguments[0]["containers"][0]["state"],
        original_state
    );
    let denied = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: "denied".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    assert!(matches!(
        reader.read(&denied, &containers, Some(&shared)).await,
        Err(RealtimeReadError::Authorization)
    ));
    for name in ["deployments", "stacks"] {
        let result = reader
            .read(
                &f.administrator,
                &Group::parse(name).unwrap(),
                Some(&change),
            )
            .await
            .unwrap();
        assert!(
            result.rows.is_empty() && result.events.is_empty(),
            "unmanaged containers should not refresh {name}"
        );
    }
    hub.publish_container_changes(f.platform_id, "update", &ids);
    let fresh = first.recv().await.unwrap();
    let refreshed = reader
        .read(&f.administrator, &containers, Some(&fresh))
        .await
        .unwrap();
    assert_eq!(
        refreshed.events[0].arguments[0]["containers"][0]["state"],
        "Exited"
    );
    let deployment = Uuid::now_v7();
    let spec: citadel_deployments::DeploymentSpec = serde_json::from_value(json!({"image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"}})).unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,$4,'Running','Idle',$5)").bind(deployment).bind(format!("batch-{deployment}")).bind(f.platform_id).bind(spec.to_storage_value().unwrap()).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let stack = Uuid::now_v7();
    let spec: citadel_stacks::StackSpec =
        serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services: {}"})).unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,controlstate) VALUES($1,$2,$3,'WebEditor',$5,$4,'Idle')").bind(stack).bind(format!("batch-{stack}")).bind(SYSTEM_ACTOR_ID).bind(citadel_stacks::StackUpdateState::new(&spec).to_storage_value().unwrap()).bind(citadel_stacks::StackDriftPolicy::default().to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')").bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE containers SET deploymentid=$2 WHERE platformid=$1 AND dockercontainerid='batch-0'",
    )
    .bind(f.platform_id)
    .bind(deployment)
    .execute(&f.pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE containers SET stackid=$2 WHERE platformid=$1 AND dockercontainerid='batch-1'",
    )
    .bind(f.platform_id)
    .bind(stack)
    .execute(&f.pool)
    .await
    .unwrap();
    for removed in [false, true] {
        if removed {
            sqlx::query("DELETE FROM containers WHERE platformid=$1 AND dockercontainerid IN ('batch-0','batch-1')").bind(f.platform_id).execute(&f.pool).await.unwrap();
        }
        hub.publish_container_changes(f.platform_id, "update", &ids);
        let change = first.recv().await.unwrap();
        for (name, owner) in [("deployments", deployment), ("stacks", stack)] {
            let result = reader
                .read(
                    &f.administrator,
                    &Group::parse(name).unwrap(),
                    Some(&change),
                )
                .await
                .unwrap();
            assert!(
                result
                    .rows
                    .iter()
                    .flat_map(|batch| &batch.rows)
                    .any(|row| row["id"] == json!(owner)),
                "{name} must refresh on owned container changes, including deletion"
            );
        }
    }
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_changes_read_only_the_scoped_projection_without_contacting_docker() {
    let f = fixture().await;
    let reader = reader(&f);
    // Any accidental network/volume inventory refresh now fails instead of
    // allowing this regression to hide behind a working Docker endpoint.
    f.docker_server.abort();
    let docker_id: String =
        sqlx::query_scalar("SELECT dockercontainerid FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let hub = citadel_server::realtime::RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut changes = hub.subscribe();
    hub.publish_runtime_change(f.platform_id, "container", "start", &docker_id);
    let change = changes.recv().await.unwrap();
    let daemon = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    let snapshot = reader
        .read(&f.administrator, &daemon, Some(&change))
        .await
        .unwrap();
    assert!(
        snapshot.events.is_empty(),
        "container changes must not refresh Swarm inventory"
    );
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].target, "ContainerEventReceived");
    assert_eq!(snapshot.rows[0].rows.len(), 1);
    assert_eq!(snapshot.rows[0].rows[0]["containerId"], docker_id);
    let containers = Group::parse(&format!("containers:{}", f.platform_id)).unwrap();
    let snapshot = reader
        .read(&f.administrator, &containers, Some(&change))
        .await
        .unwrap();
    assert!(snapshot.rows.is_empty());
    assert_eq!(snapshot.events.len(), 1);
    assert_eq!(snapshot.events[0].target, "ContainersChanged");
    assert_eq!(
        snapshot.events[0].arguments[0]["containers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let other = Group::parse(&format!("docker-daemon:{}", Uuid::now_v7())).unwrap();
    assert!(
        reader
            .read(&f.administrator, &other, Some(&change))
            .await
            .unwrap()
            .rows[0]
            .rows
            .is_empty()
    );
    // A disappeared container produces an empty scoped patch, not a full refresh.
    hub.publish_runtime_change(f.platform_id, "container", "destroy", "missing-container");
    let missing = changes.recv().await.unwrap();
    assert!(
        reader
            .read(&f.administrator, &daemon, Some(&missing))
            .await
            .unwrap()
            .rows[0]
            .rows
            .is_empty()
    );
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_logs_flow_while_another_group_snapshot_is_pending() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::containers::v1::ContainerLogResponse;
    use prost::Message as _;
    let f = fixture().await;
    let id: Uuid = sqlx::query_scalar(
        "UPDATE containers SET dockernodeid='node-1' WHERE platformid=$1 RETURNING id",
    )
    .bind(f.platform_id)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    let (session, mut commands) = f
        .lookup_state
        .platforms
        .edge
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(30),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader(&f)));
    let hub = service.hub();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&f.administrator)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    let group = format!("container-log:{id}");
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    assert!(invoke(&mut socket, "StartContainerLogs", &id.to_string()).await["error"].is_null());
    let command = commands.recv().await.unwrap();
    let command_id = Uuid::parse_str(&command.command_id).unwrap();
    let info = format!("container-info:{id}");
    assert!(invoke(&mut socket, "JoinGroup", &info).await["error"].is_null());
    assert_eq!(receive(&mut socket).await["target"], "ReceiveContainerInfo");

    let mut writer = f.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE containers IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "container", "update", id.to_string());
    // Wait for the snapshot read to actually block before producing a log.
    tokio::time::timeout(StdDuration::from_secs(2), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock')")
                .fetch_one(&f.pool).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
    }).await.expect("the shared connection must be reading its snapshot");
    let bytes = b"2026-09-08T12:00:00Z live during inventory read\n".to_vec();
    session.output(
        command_id,
        ContainerLogResponse { log: bytes.clone() }.encode_to_vec(),
    );
    let output = tokio::time::timeout(StdDuration::from_secs(1), receive(&mut socket))
        .await
        .expect("a pending snapshot must not block live log delivery");
    assert_eq!(output["target"], "SendContainerLogs");
    assert_eq!(output["arguments"][0], json!(bytes));
    writer.rollback().await.unwrap();
    assert_eq!(receive(&mut socket).await["target"], "ReceiveContainerInfo");
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

// Ports ExecSessionManagerTests and the Terminal-specific Swarm permission contract.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn terminal_websocket_requires_join_and_terminal_permission_and_owns_its_session() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::*,
        edge::v1::{EdgeCommandKind, core_envelope},
    };
    use citadel_primitives::SpecificPermission;
    use prost::Message as _;
    let f = fixture().await;
    let id:Uuid=sqlx::query_scalar("UPDATE containers SET dockernodeid='node-1',dockercontainerid=$2 WHERE platformid=$1 RETURNING id")
        .bind(f.platform_id).bind("a4c05df3937c5d2479d48cbf6b15eb85e719c27c30205fc7c9b2f82b9b973750").fetch_one(&f.pool).await.unwrap();
    let principal = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let group = format!("container-exec:{id}:browser-session");
    let reader = reader(&f);
    assert!(
        reader
            .read(&principal, &Group::parse(&group).unwrap(), None)
            .await
            .is_err()
    );
    sqlx::query("UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2")
        .bind(SpecificPermission::Terminal as i32)
        .bind(principal.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    let registry = f.lookup_state.platforms.edge.clone();
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_, mut other_node) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let cancellation = CancellationToken::new();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_millis(100),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&principal)}).to_string().into())).await.unwrap();
    receive(&mut socket).await;
    let start = json!([id, "browser-session", "sh"]);
    assert!(invoke_args(&mut socket, "StartExecProcess", start.clone()).await["error"].is_string());
    assert!(commands.try_recv().is_err());
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    assert!(invoke_args(&mut socket, "StartExecProcess", start.clone()).await["error"].is_null());
    let command = commands.recv().await.unwrap();
    let command_id = Uuid::parse_str(&command.command_id).unwrap();
    let Some(core_envelope::Body::Command(open)) = command.body else {
        panic!("exec command")
    };
    assert_eq!(open.kind, EdgeCommandKind::ContainerExec as i32);
    assert_eq!(open.node_id, "node-1");
    assert!(
        matches!(ExecClientMessage::decode(open.payload.as_slice()).unwrap().msg,Some(exec_client_message::Msg::Open(open)) if open.cmd==["/bin/sh"])
    );
    assert!(invoke_args(&mut socket, "StartExecProcess", start).await["error"].is_null());
    assert!(commands.try_recv().is_err());
    assert!(
        invoke_args(
            &mut socket,
            "SendExecInput",
            json!([id, "different-session", [65]])
        )
        .await["error"]
            .is_string()
    );
    // Knowing another connection's group/session name does not grant control.
    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    second.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&principal)}).to_string().into())).await.unwrap();
    receive(&mut second).await;
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_null());
    assert!(
        invoke_args(
            &mut second,
            "SendExecInput",
            json!([id, "browser-session", [66]])
        )
        .await["error"]
            .is_string()
    );
    assert!(commands.try_recv().is_err());
    second.close(None).await.unwrap();
    assert!(
        invoke_args(
            &mut socket,
            "SendExecInput",
            json!([id,"browser-session",{"0":65,"1":10}])
        )
        .await["error"]
            .is_null()
    );
    let input = tokio::time::timeout(StdDuration::from_secs(2), commands.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(input.command_id, command.command_id);
    assert!(matches!(
        input.body,
        Some(core_envelope::Body::StreamInput(_))
    ));
    session.output(
        command_id,
        ExecServerMessage {
            msg: Some(exec_server_message::Msg::Output(ExecOutput {
                data: b"hello".to_vec(),
                stream: 0,
            })),
        }
        .encode_to_vec(),
    );
    let event = receive(&mut socket).await;
    assert_eq!(event["target"], "SendContainerExec");
    assert_eq!(event["arguments"][0], json!(b"hello".to_vec()));
    assert!(other_node.try_recv().is_err());
    sqlx::query("UPDATE resourceaccesses SET specificpermissions=0 WHERE actorid=$1")
        .bind(principal.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    let closed = tokio::time::timeout(StdDuration::from_secs(3), socket.next())
        .await
        .unwrap();
    assert!(matches!(closed, Some(Ok(Message::Close(_))) | None));
    assert!(matches!(
        tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap()
            .body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    registry.remove(&session);
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn terminal_cancels_when_the_container_identity_changes_without_retargeting() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::edge::v1::core_envelope;
    use citadel_platforms::terminal::TerminalShell;
    let f = fixture().await;
    let id:Uuid=sqlx::query_scalar("UPDATE containers SET dockernodeid='node-1',dockercontainerid=$2 WHERE platformid=$1 RETURNING id")
        .bind(f.platform_id).bind("a4c05df3937c5d2479d48cbf6b15eb85e719c27c30205fc7c9b2f82b9b973750").fetch_one(&f.pool).await.unwrap();
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let hub = citadel_server::realtime::RealtimeHub::new(32, Arc::new(Metrics::default()));
    let mut reader = reader(&f);
    reader.docker.realtime = Some(hub.clone());
    let group = Group::parse(&format!("container-exec:{id}:identity-fence")).unwrap();
    let cancel = CancellationToken::new();
    let mut terminal = reader
        .terminal(&f.administrator, &group, TerminalShell::Sh, &cancel)
        .await
        .unwrap();
    let open = commands.recv().await.unwrap();
    sqlx::query("UPDATE containers SET dockercontainerid=$2 WHERE id=$1")
        .bind(id)
        .bind("b4c05df3937c5d2479d48cbf6b15eb85e719c27c30205fc7c9b2f82b9b973750")
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "container", "update", id.to_string());
    assert!(
        tokio::time::timeout(StdDuration::from_secs(3), terminal.output.next())
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    drop(terminal);
    let canceled = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(canceled.command_id, open.command_id);
    assert!(matches!(
        canceled.body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    assert!(commands.try_recv().is_err());
    registry.remove(&session);
    cleanup(f).await;
}

async fn invoke_args(socket: &mut Socket, target: &str, args: Value) -> Value {
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"invoke","invocationId":"1","target":target,"arguments":args}).to_string().into())).await.unwrap();
    let result = receive(socket).await;
    assert_eq!(result["kind"], "completion");
    result
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn stack_logs_follow_committed_container_replacement_without_replaying_unchanged_streams() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::{ContainerLogRequest, ContainerLogResponse},
        edge::v1::core_envelope,
    };
    use prost::Message as _;
    let f = fixture().await;
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    let spec: citadel_stacks::StackSpec =
        serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services: {}"})).unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,controlstate) VALUES($1,$2,$3,'WebEditor',$5,$4,'Idle')").bind(stack).bind(format!("logs-{stack}")).bind(SYSTEM_ACTOR_ID).bind(citadel_stacks::StackUpdateState::new(&spec).to_storage_value().unwrap()).bind(citadel_stacks::StackDriftPolicy::default().to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')").bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET stackid=$1,dockernodeid='node-1' WHERE platformid=$2")
        .bind(stack)
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let (session, mut commands) = f
        .lookup_state
        .platforms
        .edge
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let hub = citadel_server::realtime::RealtimeHub::new(32, Arc::new(Metrics::default()));
    let mut reader = reader(&f);
    reader.docker.realtime = Some(hub.clone());
    let cancellation = CancellationToken::new();
    let group = Group::parse(&format!("stack-log:{stack}")).unwrap();
    let mut logs = reader
        .stream(&f.administrator, &group, &cancellation)
        .await
        .unwrap()
        .unwrap();
    let command = commands.recv().await.unwrap();
    let original = Uuid::parse_str(&command.command_id).unwrap();
    // A ready log must not wait behind an inventory refresh. Model a busy
    // projection writer while both the log frame and its invalidation are ready.
    let mut projection_write = f.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE containers IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *projection_write)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "platformInventory", "reconciled", "");
    session.output(
        original,
        ContainerLogResponse {
            log: b"2026-09-06T12:00:00Z first\n".to_vec(),
        }
        .encode_to_vec(),
    );
    let event = tokio::time::timeout(StdDuration::from_secs(1), logs.next())
        .await
        .expect("ready logs must not wait for inventory projection reads")
        .unwrap()
        .unwrap();
    assert_eq!(event.target, "SendStackLogs");
    assert_eq!(
        event.arguments[0],
        json!(b"2026-09-06T12:00:00Z [web] first\n".to_vec())
    );
    session.output(
        original,
        ContainerLogResponse {
            log: b"2026-09-06T12:00:00Z second\n".to_vec(),
        }
        .encode_to_vec(),
    );
    let event = tokio::time::timeout(StdDuration::from_secs(1), logs.next())
        .await
        .expect("subsequent logs must flow while a projection refresh is pending")
        .unwrap()
        .unwrap();
    assert_eq!(
        event.arguments[0],
        json!(b"2026-09-06T12:00:00Z [web] second\n".to_vec())
    );
    projection_write.rollback().await.unwrap();
    hub.publish_runtime_change(f.platform_id, "platformInventory", "reconciled", "");
    session.output(
        original,
        ContainerLogResponse {
            log: b"2026-09-06T12:00:00Z unchanged\n".to_vec(),
        }
        .encode_to_vec(),
    );
    logs.next().await.unwrap().unwrap();
    assert!(commands.try_recv().is_err());
    sqlx::query("UPDATE containers SET dockercontainerid='replacement' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "platformInventory", "reconciled", "");
    let (event, ()) = tokio::join!(logs.next(), async {
        let cancel = commands.recv().await.unwrap();
        assert_eq!(cancel.command_id, original.to_string());
        assert!(matches!(
            cancel.body,
            Some(core_envelope::Body::CancelCommand(_))
        ));
        let command = commands.recv().await.unwrap();
        let id = Uuid::parse_str(&command.command_id).unwrap();
        let Some(core_envelope::Body::Command(request)) = command.body else {
            panic!("expected replacement command")
        };
        assert_eq!(
            ContainerLogRequest::decode(request.payload.as_slice())
                .unwrap()
                .container_id,
            "replacement"
        );
        session.output(
            id,
            ContainerLogResponse {
                log: b"2026-09-06T12:00:01Z replaced\n".to_vec(),
            }
            .encode_to_vec(),
        );
    });
    assert_eq!(
        event.unwrap().unwrap().arguments[0],
        json!(b"2026-09-06T12:00:01Z [web] replaced\n".to_vec())
    );
    drop(logs);
    assert!(matches!(
        commands.recv().await.unwrap().body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    f.lookup_state.platforms.edge.remove(&session);
    cleanup(f).await;
}

// Ports ContainerLogStreamManagerTests' resource-ID/exact-node cases through
// the actual WebSocket protocol, PostgreSQL projections and Edge command queue.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_logs_route_to_the_owning_node_and_cancel_on_leave_or_permission_revocation() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::{ContainerLogRequest, ContainerLogResponse},
        edge::v1::{EdgeCommandKind, core_envelope},
    };
    use citadel_primitives::SpecificPermission;
    use prost::Message as _;
    let f = fixture().await;
    let docker_id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    let id:Uuid=sqlx::query_scalar("UPDATE containers SET dockernodeid='node-1',dockercontainerid=$2 WHERE platformid=$1 RETURNING id")
        .bind(f.platform_id).bind(&docker_id).fetch_one(&f.pool).await.unwrap();
    let principal = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let group = format!("container-log:{id}");
    let reader = reader(&f);
    assert!(
        reader
            .read(&principal, &Group::parse(&group).unwrap(), None)
            .await
            .is_err(),
        "Read alone must not permit logs"
    );
    sqlx::query("UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2")
        .bind(SpecificPermission::Logs as i32)
        .bind(principal.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    let resolved = reader
        .invocation_group(&principal, "StartContainerLogs", &[json!(&docker_id)])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.name, format!("container-log:{}", &docker_id[..12]));
    reader.read(&principal, &resolved, None).await.unwrap();
    let deployment = Uuid::now_v7();
    let spec:citadel_deployments::DeploymentSpec=serde_json::from_value(json!({"image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"}})).unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,$4,'Running','Idle',$5)").bind(deployment).bind(format!("logs-{deployment}")).bind(f.platform_id).bind(spec.to_storage_value().unwrap()).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$1,state='Exited' WHERE id=$2")
        .bind(deployment)
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    let deployment_group = reader
        .invocation_group(
            &f.administrator,
            "StartDeploymentLogs",
            &[json!(deployment)],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        deployment_group.name,
        format!("container-log:{}", &docker_id[..12]),
        "the unchanged Deployment viewer joins a short Docker-ID group"
    );
    let group = deployment_group.name;
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Deployment,
        deployment,
        0,
    )
    .await;
    sqlx::query("UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2")
        .bind(SpecificPermission::Logs as i32)
        .bind(principal.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_other, mut other_commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let cancellation = CancellationToken::new();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_millis(100),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&principal)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    assert!(
        invoke(&mut socket, "StartDeploymentLogs", &deployment.to_string()).await["error"]
            .is_string(),
        "must join an authorized group first"
    );
    for revoke in [false, true] {
        assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
        assert!(
            invoke(&mut socket, "StartDeploymentLogs", &deployment.to_string()).await["error"]
                .is_null()
        );
        let command = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let command_id = Uuid::parse_str(&command.command_id).unwrap();
        let Some(core_envelope::Body::Command(request)) = command.body else {
            panic!("expected log command")
        };
        assert_eq!(request.kind, EdgeCommandKind::ContainerLogsStream as i32);
        let request = ContainerLogRequest::decode(request.payload.as_slice()).unwrap();
        assert_eq!(request.container_id, docker_id);
        assert_eq!(request.tail, 100);
        assert_eq!(request.follow, Some(true));
        assert!(other_commands.try_recv().is_err());
        assert!(
            invoke(&mut socket, "StartDeploymentLogs", &deployment.to_string()).await["error"]
                .is_null()
        );
        assert!(
            commands.try_recv().is_err(),
            "idempotent start must not duplicate a daemon subscription"
        );
        let bytes = b"2026-09-06T12:00:00Z hello\n".to_vec();
        session.output(
            command_id,
            ContainerLogResponse { log: bytes.clone() }.encode_to_vec(),
        );
        let output = receive(&mut socket).await;
        assert_eq!(output["target"], "SendContainerLogs");
        assert_eq!(output["arguments"][0], json!(bytes));
        if revoke {
            sqlx::query("UPDATE resourceaccesses SET specificpermissions=0 WHERE actorid=$1")
                .bind(principal.actor_id.value())
                .execute(&f.pool)
                .await
                .unwrap();
            let closed = tokio::time::timeout(StdDuration::from_secs(3), socket.next())
                .await
                .unwrap();
            assert!(matches!(closed, Some(Ok(Message::Close(_))) | None));
        } else {
            assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
        }
        let cancel = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancel.command_id, command_id.to_string());
        assert!(matches!(
            cancel.body,
            Some(core_envelope::Body::CancelCommand(_))
        ));
    }
    registry.remove(&session);
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn managed_service_group_requires_parent_platform_and_preserves_persisted_spec() {
    let f = fixture().await;
    let principal = super::lookup::subject(&f).await;
    let id = Uuid::now_v7();
    let spec: citadel_swarm_services::SwarmServiceSpec = serde_json::from_value(json!({
        "image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"redis","resolvedDigest":"sha256:applied"},
        "replicas":2
    })).unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat,autoupdatestate_status) VALUES($1,$2,$2,$3,$4,'hash','Unknown',$5,'Unknown',now(),'Unknown')")
        .bind(id).bind(format!("service-{id}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    let groups = reader(&f);
    let group = Group::parse(&format!("swarm-service:{id}")).unwrap();
    assert!(groups.read(&principal, &group, None).await.is_err());
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::SwarmService,
        id,
        0,
    )
    .await;
    assert!(
        groups.read(&principal, &group, None).await.is_err(),
        "Service Read alone must not bypass parent Platform visibility"
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let snapshot = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(snapshot.rows[0].target, "SwarmServiceInfoUpdated");
    let row = &snapshot.rows[0].rows[0];
    assert_eq!(row["spec"]["image"]["$type"], "External");
    assert_eq!(row["spec"]["image"]["resolvedDigest"], "sha256:applied");
    assert_eq!(row["spec"]["replicas"], 2);
    sqlx::query("UPDATE swarmservices SET spec=jsonb_set(spec,'{Replicas}','3') WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    let changed = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(changed.rows[0].rows[0]["spec"]["replicas"], 3);
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn dotnet_group_permission_matrix_uses_current_database_permissions() {
    let f = fixture().await;
    let principal = super::lookup::subject(&f).await;
    let groups = reader(&f);
    for name in [
        format!("containers:{}", f.platform_id),
        "platforms".into(),
        "deployments".into(),
        format!("activity:User:{}", principal.subject_id),
    ] {
        assert!(
            groups
                .read(&principal, &Group::parse(&name).unwrap(), None)
                .await
                .is_err(),
            "{name}"
        );
    }
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let group = Group::parse(&format!("containers:{}", f.platform_id)).unwrap();
    let snapshot = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(snapshot.events[0].target, "ContainersInfoUpdated");
    assert_eq!(
        snapshot.events[0].arguments[0]["containers"][0]["name"],
        "web"
    );
    let daemon = groups
        .read(
            &principal,
            &Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap(),
            None,
        )
        .await
        .unwrap();
    let inventory = daemon
        .events
        .iter()
        .find(|event| event.target == "SwarmInventoryUpdated")
        .unwrap();
    for kind in [
        "nodes", "services", "tasks", "networks", "configs", "secrets",
    ] {
        assert!(
            inventory.arguments[0][kind]["items"].is_array(),
            "{kind} must use the existing SwarmItemsView wrapper"
        );
    }
    assert!(
        daemon
            .rows
            .iter()
            .any(|batch| batch.target == "VolumeEventReceived")
    );
    assert!(
        daemon
            .rows
            .iter()
            .any(|batch| batch.target == "NetworkEventReceived")
    );
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(principal.actor_id.value())
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(groups.read(&principal, &group, None).await.is_err());
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn group_wire_acceptance_joins_once_delivers_committed_rows_and_leaves_without_refetch() {
    let f = fixture().await;
    let cancellation = CancellationToken::new();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(1),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader(&f)));
    let hub = service.hub();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&f.administrator)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    let group = format!("containers:{}", f.platform_id);
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    let restricted = super::lookup::subject(&f).await;
    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    second.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&restricted)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut second).await["kind"], "subscribed");
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_string());
    super::lookup::grant(
        &f,
        restricted.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_null());
    assert_eq!(
        receive(&mut second).await["target"],
        "ContainersInfoUpdated"
    );
    let initial = receive(&mut socket).await;
    assert_eq!(initial["target"], "ContainersInfoUpdated");
    assert!(initial["arguments"][0]["containers"].is_array());
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    sqlx::query("UPDATE containers SET name='renamed-live' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "container", "rename", "container-1");
    let changed = receive(&mut socket).await;
    assert_eq!(
        changed["arguments"][0]["containers"][0]["name"],
        "renamed-live"
    );
    assert_eq!(
        receive(&mut second).await["arguments"],
        changed["arguments"]
    );
    // A resource ACL removal must affect an existing authenticated socket,
    // including a repeated join. A cached token role is insufficient.
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(restricted.actor_id.value())
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_string());
    // Existing statistics handlers require Citadel IDs, not Docker IDs.
    hub.publish_container_stats(
        f.platform_id,
        &[citadel_platforms::RuntimeContainerStat {
            docker_container_id: "container-1".into(),
            cpu_usage: 42.0,
            memory_active: 10.0,
            memory_cache: 2.0,
            memory_limit: 128.0,
            rx_bytes: 0.0,
            tx_bytes: 0.0,
            created: 2,
        }],
    );
    let stats = receive(&mut socket).await;
    let revoked = tokio::time::timeout(StdDuration::from_secs(5), second.next())
        .await
        .unwrap();
    assert!(
        matches!(revoked, Some(Ok(Message::Close(_))) | None),
        "revoked subscriber received {revoked:?}"
    );
    assert_eq!(stats["target"], "ContainersStatsUpdated");
    assert_eq!(
        stats["arguments"][0][0]["containerId"],
        initial["arguments"][0]["containers"][0]["id"]
    );
    assert_eq!(stats["arguments"][0][0]["cpuUsage"], 42.0);
    assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    hub.publish_runtime_change(f.platform_id, "container", "stop", "container-1");
    // A completion acts as an ordering barrier: no event from the left group.
    assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    for denied in [
        "unknown",
        "activity:User",
        "alert-events:private",
        "container-log:container-1",
    ] {
        assert!(invoke(&mut socket, "JoinGroup", denied).await["error"].is_string());
    }
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
