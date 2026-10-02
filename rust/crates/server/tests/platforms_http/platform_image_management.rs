use super::*;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
use citadel_server::api::routes::platforms::AgentSetupContext;

async fn daemon(
    handler: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> (DockerClient, tokio::task::JoinHandle<()>, PathBuf) {
    let path = std::env::temp_dir().join(format!("platform-management-{}.sock", Uuid::now_v7()));
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = Vec::new();
            let mut buf = [0; 4096];
            while !data.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..n]);
            }
            let request = String::from_utf8_lossy(&data);
            let line = request.lines().next().unwrap_or_default();
            let (status, body) = if line.starts_with("GET /version ") {
                (
                    200,
                    json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"}).to_string(),
                )
            } else {
                handler(line)
            };
            let reply = format!(
                "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(reply.as_bytes()).await;
        }
    });
    (
        DockerClient::new(&path, StdDuration::from_secs(2)).unwrap(),
        server,
        path,
    )
}

// Ports PlatformEndpointTests rename/patch persistence, immutable orchestration/cluster,
// and RotateAgentHubKey authentication. Actual HTTP + SQL, not a catalog-only assertion.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_header_config_and_key_rotation_preserve_authorization_and_state() {
    let mut f = fixture().await;
    let cluster = format!("cluster-{}", f.platform_id);
    let daemon_id = format!("daemon-{}", f.platform_id);
    sqlx::query("UPDATE platforms SET clusterid=$2 WHERE id=$1")
        .bind(f.platform_id)
        .bind(&cluster)
        .execute(&f.pool)
        .await
        .unwrap();
    let (docker,server,socket)=daemon(move |line| {
        if line.contains("/containers/json") { return (200,"[]".into()); }
        assert!(line.contains("/info")||line.contains("/swarm"),"{line}");
        if line.contains("/swarm") {(200,json!({"ID":cluster,"CreatedAt":"2026-01-01T00:00:00Z"}).to_string())}
        else {(200,json!({"ID":daemon_id,"NCPU":2,"MemTotal":1048576,"OSType":"linux","Swarm":{"NodeID":"node-1","LocalNodeState":"active","ControlAvailable":true,"Nodes":1,"Managers":1,"Cluster":{"ID":cluster}}}).to_string())}
    }).await;
    let mut state = f.lookup_state.platforms.clone();
    refresh_runtime(&mut state, &f, docker);
    let dir = std::env::temp_dir().join(format!("agent-rotation-{}", Uuid::now_v7()));
    let key = dir.join("signing-key");
    let signer = AgentRequestSigner::load_or_create(&key).unwrap();
    let clone = signer.clone();
    let old = signer.public_key_base64();
    let context = AgentSetupContext {
        signer: Arc::new(signer),
        image: "agent:test".into(),
        requires_tls: true,
    };
    f.app = platforms_http::router(state)
        .layer(axum::Extension(Arc::new(context.view())))
        .layer(axum::Extension(context));
    let name = format!("renamed-{}", Uuid::now_v7().simple());
    let reader = super::lookup::subject(&f).await;
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            "/api/v1/platforms/rename",
            reader.clone(),
            json!({"id":f.platform_id,"name":name})
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let renamed = json_body(
        send_json(
            &f,
            Method::POST,
            "/api/v1/platforms/rename",
            f.administrator.clone(),
            json!({"id":f.platform_id,"name":name}),
        )
        .await,
    )
    .await;
    assert_eq!(renamed["name"], name);
    let activity:String=sqlx::query_scalar("SELECT info FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformRenamed' ORDER BY createdat DESC LIMIT 1").bind(f.platform_id).fetch_one(&f.pool).await.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&activity).unwrap()["NewName"],
        name
    );
    let path = format!("/api/v1/platforms/{}", f.platform_id);
    let response = send_json(
        &f,
        Method::PATCH,
        &path,
        f.administrator.clone(),
        json!({"description":"updated","pruneHistoricalSwarmTaskContainers":false}),
    )
    .await;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let updated: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["name"], name);
    assert_eq!(updated["description"], "updated");
    assert_eq!(updated["pruneHistoricalSwarmTaskContainers"], false);
    assert_eq!(
        send_json(
            &f,
            Method::PATCH,
            &path,
            f.administrator.clone(),
            json!({"type":"Docker"})
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let unchanged: bool =
        sqlx::query_scalar("SELECT prunehistoricalswarmtaskcontainers FROM platforms WHERE id=$1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert!(!unchanged);
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            "/api/v1/platforms/agent/setup/rotate-key",
            reader,
            json!({})
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let rotated = json_body(
        send_json(
            &f,
            Method::POST,
            "/api/v1/platforms/agent/setup/rotate-key",
            f.administrator.clone(),
            json!({}),
        )
        .await,
    )
    .await;
    assert_ne!(rotated["hubPublicKey"], old);
    assert_eq!(rotated["hubPublicKey"], clone.public_key_base64());
    assert_eq!(
        rotated["hubPublicKey"],
        AgentRequestSigner::from_file(&key)
            .unwrap()
            .public_key_base64()
    );
    let read = json_body(
        send(
            &f,
            "/api/v1/platforms/agent/setup",
            Some(f.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(read["hubPublicKey"], rotated["hubPublicKey"]);
    server.abort();
    let _ = std::fs::remove_file(socket);
    std::fs::remove_file(key).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

// Ports PlatformServiceTests prune dispatch and ImageEndpointTests stream + persistence.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn prune_and_pull_use_docker_and_persist_only_successful_pulls() {
    let mut f = fixture().await;
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = calls.clone();
    let id = format!("sha256:{}", "d".repeat(64));
    let image_id = id.clone();
    let (docker,server,socket)=daemon(move |line| {
        seen.lock().unwrap().push(line.to_owned());
        if line.starts_with("POST /v1.49/images/create") {assert!(line.contains("fromImage=nginx%3Alatest"));(200,"{\"status\":\"Pulling\",\"id\":\"layer\"}\n{\"status\":\"Done\"}\n".into())}
        else if line.starts_with("GET /v1.49/images/json") {(200,json!([{"Id":image_id,"RepoTags":["nginx:latest"],"RepoDigests":[format!("nginx@{image_id}")],"Created":1700000000,"Size":2048,"Containers":0}]).to_string())}
        else if line.starts_with("GET /v1.49/containers/json") {(200,"[]".into())}
        else if line.contains("/volumes/prune") {(200,json!({"VolumesDeleted":["unused"],"SpaceReclaimed":10}).to_string())}
        else if line.contains("/networks/prune") {(200,json!({"NetworksDeleted":["unused-net"]}).to_string())}
        else if line.contains("/images/prune") {assert!(line.contains("false"));(200,json!({"ImagesDeleted":[{"Deleted":"old-image"},{"Untagged":"old:tag"}],"SpaceReclaimed":20}).to_string())}
        else if line.contains("/build/prune") {assert!(line.contains("all=true"));(200,json!({"CachesDeleted":["cache"],"SpaceReclaimed":30}).to_string())}
        else {panic!("unexpected {line}")}
    }).await;
    let mut state = f.lookup_state.platforms.clone();
    refresh_runtime(&mut state, &f, docker);
    f.app = platforms_http::router(state);
    let path = format!("/api/v1/platforms/{}/prune", f.platform_id);
    let reader = super::lookup::subject(&f).await;
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            &path,
            reader.clone(),
            json!({"resource":"All"})
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(calls.lock().unwrap().is_empty());
    let pruned = json_body(
        send_json(
            &f,
            Method::POST,
            &path,
            f.administrator.clone(),
            json!({"resource":"All"}),
        )
        .await,
    )
    .await;
    assert_eq!(pruned["spaceReclaimed"], 60);
    assert_eq!(pruned["imagesDeleted"], json!(["old-image", "old:tag"]));
    assert_eq!(calls.lock().unwrap().len(), 4);
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            &path,
            f.administrator.clone(),
            json!({"resource":"Container"})
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let before = calls.lock().unwrap().len();
    let invalid = send_json(&f, Method::POST, "/api/v1/images/pull", f.administrator.clone(),
        json!({"platformId":f.platform_id,"registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx bad"})).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(invalid.into_body(), 1024 * 1024).await.unwrap();
    let problem: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        problem["type"]
            .as_str()
            .unwrap()
            .ends_with("validation_error")
    );
    assert_eq!(problem["detail"], "A valid image reference is required.");
    assert_eq!(
        calls.lock().unwrap().len(),
        before,
        "invalid references never dispatch Docker calls"
    );
    let body = json!({"platformId":f.platform_id,"registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"});
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            "/api/v1/images/pull",
            reader,
            body.clone()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let progress = json_body(
        send_json(
            &f,
            Method::POST,
            "/api/v1/images/pull",
            f.administrator.clone(),
            body,
        )
        .await,
    )
    .await;
    assert_eq!(progress[0]["status"], "Pulling");
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["dockerImageId"],
        id,
        "{progress}"
    );
    let stored: (Option<Uuid>, f64, Value) = sqlx::query_as(
        "SELECT registryid,size,tags::jsonb FROM images WHERE platformid=$1 AND dockerimageid=$2",
    )
    .bind(f.platform_id)
    .bind(&id)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(stored.0, Some(Uuid::from_u128(0x100)));
    assert_eq!(stored.1, 2048.0);
    assert_eq!(stored.2, json!(["nginx:latest"]));
    server.abort();
    let _ = std::fs::remove_file(socket);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_platform_prune_and_pull_stream_route_and_persist_without_local_fallback() {
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        edge::v1::{EdgeCommandKind, core_envelope},
        images::v1::{PullImageRequest, PullImageResponse},
        platforms::v1::{PruneRequest, PruneResponse},
        shared_models::v1::{ImageReply, ListImageResponse},
    };
    use prost::Message;
    let f = fixture().await;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',address=$2 WHERE id=$1")
        .bind(f.platform_id)
        .bind(format!("edge://{}", f.platform_id))
        .execute(&f.pool)
        .await
        .unwrap();
    let (session, mut commands) = f
        .edge
        .register(EdgeTarget::platform(f.platform_id), Uuid::now_v7())
        .unwrap();
    let path = format!("/api/v1/platforms/{}/prune", f.platform_id);
    let (response, ()) = tokio::join!(
        send_json(
            &f,
            Method::POST,
            &path,
            f.administrator.clone(),
            json!({"resource":"Volume"})
        ),
        async {
            let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
                .await
                .unwrap()
                .unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected prune")
            };
            assert_eq!(command.kind, EdgeCommandKind::PlatformPrune as i32);
            assert_eq!(
                PruneRequest::decode(command.payload.as_slice())
                    .unwrap()
                    .resource,
                2
            );
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            session.output(
                id,
                PruneResponse {
                    resource: 2,
                    space_reclaimed: 73,
                    volumes_deleted: vec!["edge-volume".into()],
                    ..Default::default()
                }
                .encode_to_vec(),
            );
            session.complete(id, true);
        }
    );
    assert_eq!(json_body(response).await["spaceReclaimed"], 73);
    let image_id = format!("sha256:{}", "e".repeat(64));
    let body = json!({"platformId":f.platform_id,"registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"});
    let (progress, ()) = tokio::join!(
        async {
            json_body(
                send_json(
                    &f,
                    Method::POST,
                    "/api/v1/images/pull",
                    f.administrator.clone(),
                    body,
                )
                .await,
            )
            .await
        },
        async {
            for step in 0..2 {
                let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
                    .await
                    .unwrap()
                    .unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected pull/list")
                };
                let id = Uuid::parse_str(&envelope.command_id).unwrap();
                if step == 0 {
                    assert_eq!(command.kind, EdgeCommandKind::ImagePullStream as i32);
                    let req = PullImageRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(req.from_image, "nginx:latest");
                    assert!(req.auth.is_none());
                    session.output(
                        id,
                        PullImageResponse {
                            status: Some("Pulling".into()),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                    );
                } else {
                    assert_eq!(command.kind, EdgeCommandKind::ImageList as i32);
                    session.output(
                        id,
                        ListImageResponse {
                            images: vec![ImageReply {
                                id: image_id.clone(),
                                repo_tags: vec!["nginx:latest".into()],
                                size: 4096.0,
                                ..Default::default()
                            }],
                        }
                        .encode_to_vec(),
                    );
                }
                session.complete(id, true);
            }
        }
    );
    assert_eq!(progress[0]["status"], "Pulling");
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["dockerImageId"],
        image_id,
        "{progress}"
    );
    let size: f64 =
        sqlx::query_scalar("SELECT size FROM images WHERE platformid=$1 AND dockerimageid=$2")
            .bind(f.platform_id)
            .bind(&image_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(size, 4096.0);
    assert!(commands.try_recv().is_err());
}
