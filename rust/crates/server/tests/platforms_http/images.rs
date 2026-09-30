use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    images::v1::{
        GetExposedPortsRequest, GetExposedPortsResponse, InspectImageRequest, InspectImageResponse,
    },
};
use prost::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn delete_images_preserves_authorization_and_reconciles_partial_failure() {
    let f = fixture().await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite, SnapshotGeneration};
    let containers =
        SnapshotGeneration::capture(f.platform_id, None, ProjectionKind::Containers).await;
    let images = SnapshotGeneration::capture(f.platform_id, None, ProjectionKind::Images).await;
    let first = format!("sha256:{}", "a".repeat(64));
    let second = format!("sha256:{}", "b".repeat(64));
    for image in [&first, &second] {
        sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,$2,$3,now(),'[]')").bind(Uuid::now_v7()).bind(image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    }
    let socket = std::env::temp_dir().join(format!("images-{}.sock", Uuid::now_v7()));
    let listener = UnixListener::bind(&socket).unwrap();
    let deleted = Arc::new(Mutex::new(Vec::new()));
    let operations = deleted.clone();
    let a = first.clone();
    let b = second.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let line = request.lines().next().unwrap();
            let (status, body) = if line.starts_with("GET /version ") {
                (200, json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"}))
            } else if line.starts_with("DELETE ") {
                assert!(line.contains("force=true&noprune=true"));
                operations.lock().await.push(line.to_owned());
                if line.contains(&"b".repeat(64)) {
                    (409, json!({"message":"image is in use"}))
                } else {
                    (200, json!([{"Deleted":a}]))
                }
            } else if line.starts_with("GET /v1.49/containers/json?all=true ") {
                (
                    200,
                    json!([{"Id":"stopped-container","ImageID":b,"State":"exited"}]),
                )
            } else {
                assert!(line.starts_with("GET /v1.49/images/json"));
                (
                    200,
                    json!([{"Id":b,"RepoTags":[],"RepoDigests":[],"Created":0,"Size":0,"Containers":-1}]),
                )
            };
            let body = body.to_string();
            let phrase = if status == 200 { "OK" } else { "Conflict" };
            stream.write_all(format!("HTTP/1.1 {status} {phrase}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let mut state = f.lookup_state.platforms.clone();
    state.docker = DockerClient::new(&socket, StdDuration::from_secs(2)).unwrap();
    let docker = state.docker.clone();
    let mut f = f;
    f.app = platforms_http::router(state);
    let body = json!({"platformId":f.platform_id,"ids":[first,second],"force":true,"noPrune":true});
    let anonymous = Request::builder()
        .method(Method::DELETE)
        .uri("/api/v1/images")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    assert_eq!(
        f.app.clone().oneshot(anonymous).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_json(
            &f,
            Method::DELETE,
            "/api/v1/images",
            super::lookup::subject(&f).await,
            body.clone()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(deleted.lock().await.is_empty());
    let unknown = json!({"platformId":f.platform_id,"ids":[format!("sha256:{}","c".repeat(64))]});
    assert_eq!(
        send_json(
            &f,
            Method::DELETE,
            "/api/v1/images",
            f.administrator.clone(),
            unknown
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert!(deleted.lock().await.is_empty());
    assert_eq!(
        send_json(
            &f,
            Method::DELETE,
            "/api/v1/images",
            f.administrator.clone(),
            body
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert!(
        !images.matches(&ProjectionWrite::begin(f.platform_id, None, ProjectionKind::Images).await)
    );
    assert!(
        containers.matches(
            &ProjectionWrite::begin(f.platform_id, None, ProjectionKind::Containers).await
        )
    );
    assert_eq!(deleted.lock().await.len(), 2);
    let rows:Vec<(String,String)>=sqlx::query_as("SELECT dockerimageid,controlstate FROM images WHERE platformid=$1 AND dockerimageid=ANY($2)").bind(f.platform_id).bind(&[first.clone(),second.clone()]).fetch_all(&f.pool).await.unwrap();
    assert_eq!(rows, vec![(second.clone(), "Idle".into())]);
    // Deletion confirmation removes missing identities; the inventory worker
    // owns summary fields. Exercise its Docker -> projection -> HTTP path.
    let snapshot = citadel_platforms::jobs::collect_event_scope(
        citadel_platforms::jobs::ResourceCollector::Images(&docker),
        f.platform_id,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist_resource(&snapshot)
        .await
        .unwrap();
    let response = json_body(
        send_json(
            &f,
            Method::GET,
            &format!("/api/v1/images/{}", f.platform_id),
            f.administrator.clone(),
            Value::Null,
        )
        .await,
    )
    .await;
    let image = response["images"]
        .as_array()
        .unwrap()
        .iter()
        .find(|image| image["dockerImageId"] == second)
        .unwrap();
    assert_eq!(
        image["isInUse"], true,
        "stopped containers still use their images; Docker -1 is unknown, not unused"
    );
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,$2,$3,now(),'[]')")
        .bind(Uuid::now_v7()).bind(&first).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let success = json_body(
        send_json(
            &f,
            Method::DELETE,
            "/api/v1/images",
            f.administrator.clone(),
            json!({"platformId":f.platform_id,"ids":[first,first],"force":true,"noPrune":true}),
        )
        .await,
    )
    .await;
    assert_eq!(success["items"][0]["result"]["Deleted"], first);
    assert_eq!(
        deleted.lock().await.len(),
        3,
        "Duplicate IDs must not repeat a Docker mutation"
    );
    server.abort();
    f.docker_server.abort();
    std::fs::remove_file(socket).unwrap();
}

// Ports ImageEndpointTests and SwarmNodeLocalResourceEndpointTests' inspect
// contract, adding a wrong-node/no-fallback regression at the real HTTP boundary.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn image_inspect_authorizes_and_uses_the_exact_node_with_existing_ui_shape() {
    let f = fixture().await;
    let url = format!(
        "/api/v1/images/{}/sha256:abc?dockerNodeId=node-1",
        f.platform_id
    );
    assert_eq!(
        send(&f, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let principal = super::lookup::subject(&f).await;
    assert_eq!(
        send(&f, &url, Some(principal.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    // Refresh the permission cache after the fixture grants access directly in SQL.
    super::realtime_groups::replace_specific_permissions(&f, &principal, vec![]).await;
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
    let (response, ()) = tokio::join!(send(&f, &url, Some(principal.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("image command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageInspect as i32);
        assert_eq!(
            InspectImageRequest::decode(command.payload.as_slice())
                .unwrap()
                .id,
            "sha256:abc"
        );
        let id = Uuid::parse_str(&command.command_id).unwrap();
        session.output(
            id,
            InspectImageResponse {
                id: "sha256:abc".into(),
                repo_tags: vec!["registry:5000/app:v1".into()],
                env: vec!["MODE=production".into()],
                exposed_ports: vec!["80/tcp".into()],
                ..Default::default()
            }
            .encode_to_vec(),
        );
        session.complete(id, true);
    });
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "registry:5000/app");
    assert_eq!(body["tag"], "v1");
    assert_eq!(body["dockerNodeId"], "node-1");
    assert_eq!(body["exposedPorts"], json!(["80/tcp"]));
    assert_eq!(body["capabilities"]["canRead"], true);
    assert!(body["containers"].is_array() && body["layers"].is_array());
    assert!(other_commands.try_recv().is_err());
    // Port GetExposedPortsTests at the HTTP/SQL/transport boundary: the form's
    // Citadel image UUID must resolve to its Docker content ID on this node.
    let image_id = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmnodeimageprojections(id,contentidentity,dockerimageid,dockernodeid,observedat,platformid,resource) VALUES($1,'sha256:abc','sha256:abc','node-1',now(),$2,'{}')")
        .bind(image_id).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let ports_url = format!("/api/v1/images/{}/{image_id}/_ports", f.platform_id);
    assert_eq!(
        send(&f, &ports_url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let (ports_response, ()) = tokio::join!(send(&f, &ports_url, Some(principal.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("ports command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageExposedPorts as i32);
        assert_eq!(
            GetExposedPortsRequest::decode(command.payload.as_slice())
                .unwrap()
                .id,
            "sha256:abc"
        );
        let command_id = Uuid::parse_str(&command.command_id).unwrap();
        session.output(
            command_id,
            GetExposedPortsResponse {
                ports: vec!["80/tcp".into()],
            }
            .encode_to_vec(),
        );
        session.complete(command_id, true);
    });
    assert_eq!(ports_response.status(), StatusCode::OK);
    assert_eq!(json_body(ports_response).await, json!({"ports":["80/tcp"]}));
    assert_eq!(
        send(
            &f,
            &ports_url.replace(&image_id.to_string(), &Uuid::nil().to_string()),
            Some(principal.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &f,
            &ports_url.replace(&f.platform_id.to_string(), &Uuid::now_v7().to_string()),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert!(other_commands.try_recv().is_err());
    assert_eq!(
        send(
            &f,
            &url.replace("node-1", "missing-node"),
            Some(principal.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE swarmnodeprojections SET isstale=true WHERE platformid=$1 AND dockernodeid='node-1'").bind(f.platform_id).execute(&f.pool).await.unwrap();
    assert_eq!(
        send(&f, &url, Some(principal)).await.status(),
        StatusCode::CONFLICT
    );
    assert!(commands.try_recv().is_err());
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}
