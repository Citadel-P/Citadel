use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    images::v1::{
        GetExposedPortsRequest, GetExposedPortsResponse, InspectImageRequest, InspectImageResponse,
    },
};
use citadel_platforms::image_updates::ImageUpdateTarget;
use prost::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
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
    let docker = DockerClient::new(&socket, StdDuration::from_secs(2)).unwrap();
    refresh_runtime(&mut state, &f, docker.clone());
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
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
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
    let registry = &f.edge;
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

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_deletion_review_rechecks_live_tags_and_stopped_containers() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut f = fixture().await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let id = format!("sha256:{}", "d".repeat(64));
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags,controlstate) VALUES($1,$2,'app:latest',$3,now(),'[\"app:latest\"]','Idle')").bind(Uuid::now_v7()).bind(&id).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let mode = Arc::new(AtomicUsize::new(0));
    let deletes = Arc::new(AtomicUsize::new(0));
    let daemon_mode = mode.clone();
    let dispatched = deletes.clone();
    let image_id = id.clone();
    let (docker, server, socket) = super::platform_image_management::daemon(move |line| {
        let mode = daemon_mode.load(Ordering::SeqCst);
        if line.starts_with("DELETE ") { dispatched.fetch_add(1,Ordering::SeqCst); return (200,"[]".into()); }
        if line.contains("/history") { return (200,"[]".into()); }
        if line.contains("/containers/json") { return (200, if mode == 2 {json!([{"Id":"stopped","ImageID":image_id,"Names":["/stopped"],"State":"exited"}]).to_string()} else {"[]".into()}); }
        let tags = if mode == 1 {vec!["app:other"]} else {vec!["app:latest"]};
        if line.contains("/images/json") { return (200,json!([{"Id":image_id,"RepoTags":tags,"Created":0,"Size":0,"Containers":0}]).to_string()); }
        assert!(line.contains("/images/") && line.contains("/json"), "{line}");
        (200,json!({"Id":image_id,"RepoTags":tags,"Created":"2026-10-15T00:00:00Z","Os":"linux","Architecture":"amd64","Size":1,"Config":{},"RootFS":{"Type":"layers","Layers":[]}}).to_string())
    }).await;
    let mut state = f.lookup_state.platforms.clone();
    refresh_runtime(&mut state, &f, docker);
    f.app = platforms_http::router(state);
    let preview = send_json(
        &f,
        Method::POST,
        "/api/v1/images/deletion-preview",
        f.administrator.clone(),
        json!({"platformId":f.platform_id,"ids":[id]}),
    )
    .await;
    assert_eq!(preview.status(), StatusCode::OK);
    let preview = json_body(preview).await;
    assert_eq!(preview[0]["canDelete"], true, "{preview}");
    assert_eq!(preview[0]["review"]["tags"], json!(["app:latest"]));
    for live_change in [1, 2] {
        mode.store(live_change, Ordering::SeqCst);
        // Refresh the persisted version after a previous claim was released.
        let version: i64 = sqlx::query_scalar(
            "SELECT rowversion FROM images WHERE platformid=$1 AND dockerimageid=$2",
        )
        .bind(f.platform_id)
        .bind(&id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        let response = send_json(&f,Method::DELETE,"/api/v1/images",f.administrator.clone(),json!({"platformId":f.platform_id,"ids":[id],"force":false,"noPrune":true,"reviews":[{"dockerImageId":id,"rowVersion":version,"tags":["app:latest"]}]})).await;
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "{}",
            json_body(response).await
        );
        assert_eq!(deletes.load(Ordering::SeqCst), 0);
    }
    let preview = json_body(
        send_json(
            &f,
            Method::POST,
            "/api/v1/images/deletion-preview",
            f.administrator.clone(),
            json!({"platformId":f.platform_id,"ids":[id]}),
        )
        .await,
    )
    .await;
    assert_eq!(preview[0]["canDelete"], false);
    assert!(!preview[0]["blockers"].as_array().unwrap().is_empty());
    server.abort();
    let _ = server.await;
    let _ = tokio::fs::remove_file(socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_update_checks_are_fresh_metadata_only_and_list_payload_is_authorized() {
    use citadel_contracts::citadel::images::v1::{
        DistributionInspectRequest, DistributionInspectResponse, OciDescriptorResponse,
    };
    let mut f = fixture().await;
    use citadel_server::realtime_groups::{Group, GroupReadPort};
    let limited = super::lookup::subject(&f).await;
    let group = Group::parse(&format!("images:{}", f.platform_id)).unwrap();
    let reader = super::realtime_groups::reader(&f);
    assert!(reader.read(&limited, &group, None).await.is_err());
    super::lookup::grant(
        &f,
        limited.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;

    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',platformdescriptor='{\"$type\":\"Docker\"}',status='Online' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let image = format!("sha256:{}", "a".repeat(64));
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx:17',$3,now(),'[\"nginx:17\",\"nginx:stable\"]')").bind(Uuid::now_v7()).bind(&image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let mut state = f.lookup_state.platforms.clone();
    let runtime =
        citadel_adapters::connectors::routing::platforms::runtime::PlatformRuntimeRouter::new(
            f.pool.clone(),
            f.docker.clone(),
            f.agent.clone(),
            f.edge.clone(),
        );
    state.image_updates = Arc::new(citadel_platforms::image_updates::ImageUpdateService::new(
        Arc::new(runtime.clone()),
        Arc::new(
            citadel_adapters::connectors::registries::image_updates::ImageUpdateAdapter {
                pool: f.pool.clone(),
                runtime,
                cache: Arc::new(Default::default()),
            },
        ),
     Arc::new(citadel_adapters::persistence::postgres::platforms::image_updates::PostgresImageUpdateStore(f.pool.clone())), ));
    f.app = platforms_http::router(state);
    let (session, mut commands) = f
        .edge
        .register(EdgeTarget::platform(f.platform_id), Uuid::now_v7())
        .unwrap();
    let input = json!({"platformId":f.platform_id,"dockerImageId":image,"registryId":Uuid::from_u128(0x100),"reference":"nginx:17"});
    let mut last_success = Value::Null;
    for (config, status) in [
        (image.clone(), "UpToDate"),
        (format!("sha256:{}", "b".repeat(64)), "UpdateAvailable"),
        (String::new(), "Unknown"),
    ] {
        let expected = if config.is_empty() { 2 } else { 3 };
        let (response, ()) = tokio::join!(
            send_json(
                &f,
                Method::POST,
                "/api/v1/images/check-update",
                f.administrator.clone(),
                input.clone()
            ),
            async {
                for i in 0..expected {
                    let envelope = tokio::time::timeout(StdDuration::from_secs(5), commands.recv())
                        .await
                        .unwrap()
                        .unwrap();
                    let Some(core_envelope::Body::Command(command)) = envelope.body else {
                        panic!("metadata command")
                    };
                    let command_id = Uuid::parse_str(&command.command_id).unwrap();
                    let bytes = if i == 1 {
                        assert_eq!(
                            command.kind,
                            EdgeCommandKind::ImageDistributionInspect as i32,
                            "checks must never pull or apply workloads"
                        );
                        let request =
                            DistributionInspectRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.image_name, "nginx:17");
                        let target = request.target_platform.unwrap();
                        assert_eq!(target.os, "linux");
                        assert_eq!(target.architecture, "amd64");
                        DistributionInspectResponse {
                            config_digest: config.clone(),
                            manifest_digest: format!("sha256:{}", "c".repeat(64)),
                            descriptor: Some(OciDescriptorResponse {
                                digest: format!("sha256:{}", "d".repeat(64)),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                        .encode_to_vec()
                    } else {
                        assert_eq!(command.kind, EdgeCommandKind::ImageInspect as i32);
                        let request =
                            InspectImageRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.id, image);
                        InspectImageResponse {
                            config_digest: Some(image.clone()),
                            id: image.clone(),
                            repo_tags: vec!["nginx:17".into(), "nginx:stable".into()],
                            os: Some("linux".into()),
                            architecture: Some("amd64".into()),
                            ..Default::default()
                        }
                        .encode_to_vec()
                    };
                    session.output(command_id, bytes);
                    session.complete(command_id, true);
                }
            }
        );
        let code = response.status();
        let body = json_body(response).await;
        assert_eq!(code, StatusCode::OK, "{body}");
        assert_eq!(body["status"], status, "{body}");
        if status == "Unknown" {
            assert_eq!(body["lastSuccessAt"], last_success);
            assert_eq!(body["isStale"], true);
        } else {
            last_success = body["lastSuccessAt"].clone();
        }
        let response = send(
            &f,
            &format!("/api/v1/images/{}", f.platform_id),
            Some(f.administrator.clone()),
        )
        .await;
        let code = response.status();
        let list = json_body(response).await;
        assert_eq!(code, StatusCode::OK, "{list}");
        let row = list["images"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["dockerImageId"] == image)
            .unwrap();
        assert_eq!(row["updates"][0]["status"], status);
        assert_eq!(row["updates"][1]["reference"], "nginx:stable");
        assert_eq!(row["updates"][1]["status"], "NotChecked");
        assert!(row["usedBy"]["deployments"].is_number());
        // HTTP and realtime decorate the same inventory for each actor. A
        // subscriber without Registry access must not receive its observation.
        for principal in [&f.administrator, &limited] {
            let http = json_body(
                send(
                    &f,
                    &format!("/api/v1/images/{}", f.platform_id),
                    Some(principal.clone()),
                )
                .await,
            )
            .await;
            let snapshot = reader.read(principal, &group, None).await.unwrap();
            assert_eq!(snapshot.events.len(), 1);
            assert_eq!(snapshot.events[0].target, "ImagesInfoUpdated");
            let live = &snapshot.events[0].arguments[0];
            assert_eq!(live["platformId"], json!(f.platform_id));
            assert_eq!(live["images"], http["images"]);
            assert_eq!(live["capabilities"], http["capabilities"]);
            if principal.actor_id == limited.actor_id {
                assert_eq!(live["capabilities"]["canPull"], false);
                let row = live["images"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["dockerImageId"] == image)
                    .unwrap();
                assert_eq!(row["updates"][0]["status"], "NotChecked");
                assert!(row["updates"][0]["remoteConfigDigest"].is_null());
            }
        }
        if status == "UpToDate" {
            sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
                .bind(f.platform_id)
                .execute(&f.pool)
                .await
                .unwrap();
            let stale = json_body(
                send(
                    &f,
                    &format!("/api/v1/images/{}", f.platform_id),
                    Some(f.administrator.clone()),
                )
                .await,
            )
            .await;
            let stale_image = stale["images"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["dockerImageId"] == image)
                .unwrap();
            assert_eq!(stale_image["updates"][0]["status"], "Unknown");
            assert_eq!(stale_image["updates"][0]["isStale"], true);
            sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
                .bind(f.platform_id)
                .execute(&f.pool)
                .await
                .unwrap();
        }
        assert!(
            commands.try_recv().is_err(),
            "passive list reads must not contact Docker or a Registry"
        );
    }
    let mut denied = input.clone();
    denied["registryId"] = json!(Uuid::now_v7());
    // An inaccessible Registry never sends an authenticated metadata request.
    let response = send_json(
        &f,
        Method::POST,
        "/api/v1/images/check-update",
        f.administrator.clone(),
        denied,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(commands.try_recv().is_err());
    f.docker_server.abort();
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn scheduled_image_checks_follow_pulled_tags_and_survive_restart() {
    use citadel_adapters::persistence::postgres::platforms::image_updates::PostgresImageUpdateStore;
    use citadel_contracts::citadel::images::v1::{
        DistributionInspectRequest, DistributionInspectResponse,
    };
    use citadel_platforms::image_updates::{ImageUpdateService, ImageUpdateStatus};
    use std::sync::atomic::Ordering;
    use tokio_util::sync::CancellationToken;
    let f = fixture().await;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',platformdescriptor='{\"$type\":\"Docker\"}',status='Online' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    // Only the synthetic image is eligible; fixture inventory is not part of this sweep.
    sqlx::query("UPDATE images SET tags='[]' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let old = format!("sha256:{}", "a".repeat(64));
    let latest = format!("sha256:{}", "b".repeat(64));
    let row = Uuid::now_v7();
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags,registryid) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:17\"]','00000000-0000-0000-0000-000000000100')").bind(row).bind(&old).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let make_service = || {
        let runtime =
            citadel_adapters::connectors::routing::platforms::runtime::PlatformRuntimeRouter::new(
                f.pool.clone(),
                f.docker.clone(),
                f.agent.clone(),
                f.edge.clone(),
            );
        ImageUpdateService::new(
            Arc::new(runtime.clone()),
            Arc::new(
                citadel_adapters::connectors::registries::image_updates::ImageUpdateAdapter {
                    pool: f.pool.clone(),
                    runtime,
                    cache: Arc::new(Default::default()),
                },
            ),
            Arc::new(PostgresImageUpdateStore(f.pool.clone())),
        )
    };
    let changes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let notifications = changes.clone();
    let service = make_service().with_change_handler(Arc::new(move |_, _| {
        notifications.fetch_add(1, Ordering::SeqCst);
    }));
    let (session, mut commands) = f
        .edge
        .register(EdgeTarget::platform(f.platform_id), Uuid::now_v7())
        .unwrap();
    let cancel = CancellationToken::new();
    for (attempt, (image, expected)) in [
        (&old, ImageUpdateStatus::UpdateAvailable),
        (&latest, ImageUpdateStatus::UpToDate),
        (&latest, ImageUpdateStatus::UpToDate),
    ]
    .into_iter()
    .enumerate()
    {
        if attempt == 1 {
            // Inventory after a Deployment/Compose pull: the same tag now belongs
            // to a different image; the old content may remain on the host.
            use citadel_platforms::jobs::{ProjectionKind, ResourceDelta, SnapshotGeneration};
            let projections = citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore::new(f.pool.clone());
            let stamp =
                SnapshotGeneration::capture(f.platform_id, None, ProjectionKind::Images).await;
            projections
                .persist_delta(
                    f.platform_id,
                    &ResourceDelta::Image {
                        id: latest.clone(),
                        value: Some(citadel_platforms::RuntimeImageSummary {
                            id: latest.clone(),
                            repo_tags: vec!["nginx:17".into()],
                            ..Default::default()
                        }),
                    },
                    &stamp,
                )
                .await
                .unwrap();
        }
        if attempt == 2 {
            sqlx::query("UPDATE imageupdateobservations SET attemptedat=now()-interval '3 hours', result=jsonb_set(result,'{lastAttemptAt}',to_jsonb(now()-interval '3 hours')) WHERE imageid IN (SELECT id FROM images WHERE platformid=$1 AND dockerimageid=$2)")
                .bind(f.platform_id).bind(&latest).execute(&f.pool).await.unwrap();
        }
        let (batch, ()) = tokio::join!(service.run_scheduled_batch(Uuid::nil(), &cancel), async {
            for step in 0..3 {
                let envelope = tokio::time::timeout(StdDuration::from_secs(10), commands.recv())
                    .await
                    .unwrap()
                    .unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("metadata command")
                };
                let command_id = Uuid::parse_str(&command.command_id).unwrap();
                let bytes = if step == 1 {
                    assert_eq!(
                        command.kind,
                        EdgeCommandKind::ImageDistributionInspect as i32
                    );
                    let request =
                        DistributionInspectRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.image_name, "nginx:17");
                    DistributionInspectResponse {
                        config_digest: latest.clone(),
                        manifest_digest: format!("sha256:{}", "c".repeat(64)),
                        ..Default::default()
                    }
                    .encode_to_vec()
                } else {
                    assert_eq!(
                        command.kind,
                        EdgeCommandKind::ImageInspect as i32,
                        "scheduled checks never pull or deploy"
                    );
                    assert_eq!(
                        InspectImageRequest::decode(command.payload.as_slice())
                            .unwrap()
                            .id,
                        *image
                    );
                    InspectImageResponse {
                        config_digest: Some(image.clone()),
                        id: image.clone(),
                        repo_tags: vec!["nginx:17".into()],
                        os: Some("linux".into()),
                        architecture: Some("amd64".into()),
                        ..Default::default()
                    }
                    .encode_to_vec()
                };
                session.output(command_id, bytes);
                session.complete(command_id, true);
            }
        });
        let batch = batch.unwrap();
        assert_eq!(batch.checked, 1);
        assert_eq!(batch.failed, 0);
        let inventory = f
            .lookup_state
            .platforms
            .platforms
            .list_images(f.platform_id)
            .await
            .unwrap();
        let restarted = make_service();
        let summaries = restarted
            .summaries(f.administrator.actor_id, true, f.platform_id, &inventory)
            .await
            .unwrap();
        let index = inventory
            .iter()
            .position(|i| i.docker_image_id == *image)
            .unwrap();
        assert_eq!(summaries[index][0].status, expected);
        assert_eq!(
            restarted
                .run_scheduled_batch(Uuid::nil(), &cancel)
                .await
                .unwrap()
                .checked,
            0,
            "restart and repeated scans respect the cadence"
        );
        if image == &latest {
            let index = inventory
                .iter()
                .position(|i| i.docker_image_id == old)
                .unwrap();
            assert_eq!(summaries[index][0].status, ImageUpdateStatus::NotApplicable);
        }
    }
    assert_eq!(changes.load(Ordering::SeqCst), 3);
    cancel.cancel();
    assert!(
        service
            .run_scheduled_batch(Uuid::nil(), &cancel)
            .await
            .is_err()
    );
    assert!(
        commands.try_recv().is_err(),
        "no pull or deployment commands"
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn no_op_pull_refreshes_cached_status_and_metadata_failure_does_not_fail_the_pull() {
    use citadel_contracts::citadel::{
        images::v1::{DistributionInspectResponse, PullImageResponse},
        shared_models::v1::{ImageReply, ListImageResponse},
    };
    use citadel_platforms::image_updates::ImageUpdateStatus;
    use citadel_platforms::image_usage::ImageLocation;
    let f = fixture().await;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let image = format!("sha256:{}", "a".repeat(64));
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:17\"]')").bind(Uuid::now_v7()).bind(&image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let (session, mut commands) = f
        .edge
        .register(EdgeTarget::platform(f.platform_id), Uuid::now_v7())
        .unwrap();
    let cancel = CancellationToken::new();
    let mut last_attempt = None;
    for (phase, count) in [3, 6, 5].into_iter().enumerate() {
        let (result, ()) = tokio::join!(
            async {
                if phase == 0 {
                    serde_json::to_value(
                        f.lookup_state
                            .platforms
                            .image_updates
                            .check(
                                f.administrator.actor_id,
                                true,
                                ImageUpdateTarget {
                                    platform: f.platform_id,
                                    location: &ImageLocation {
                                        docker_image_id: image.clone(),
                                        docker_node_id: None,
                                    },
                                    reference: "nginx:17",
                                },
                                Uuid::from_u128(0x100),
                                &cancel,
                            )
                            .await
                            .unwrap(),
                    )
                    .unwrap()
                } else {
                    json_body(send_json(&f, Method::POST, "/api/v1/images/pull", f.administrator.clone(),
                    json!({"platformId":f.platform_id,"registryId":Uuid::from_u128(0x100),"sourceImageId":image,"imageTag":"nginx:17"})
                ).await).await
                }
            },
            async {
                for _ in 0..count {
                    let envelope =
                        tokio::time::timeout(StdDuration::from_secs(10), commands.recv())
                            .await
                            .unwrap()
                            .unwrap();
                    let Some(core_envelope::Body::Command(command)) = envelope.body else {
                        panic!("image command")
                    };
                    let id = Uuid::parse_str(&command.command_id).unwrap();
                    let bytes = match EdgeCommandKind::try_from(command.kind).unwrap() {
                        EdgeCommandKind::ImageInspect => InspectImageResponse {
                            config_digest: Some(image.clone()),
                            id: image.clone(),
                            repo_tags: vec!["nginx:17".into()],
                            os: Some("linux".into()),
                            architecture: Some("amd64".into()),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                        EdgeCommandKind::ImageDistributionInspect => DistributionInspectResponse {
                            config_digest: match phase {
                                0 => format!("sha256:{}", "b".repeat(64)),
                                1 => image.clone(),
                                _ => String::new(),
                            },
                            manifest_digest: format!("sha256:{}", "c".repeat(64)),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                        EdgeCommandKind::ImagePullStream => PullImageResponse {
                            status: Some("Image is up to date".into()),
                            ..Default::default()
                        }
                        .encode_to_vec(),
                        EdgeCommandKind::ImageList => ListImageResponse {
                            images: vec![ImageReply {
                                id: image.clone(),
                                repo_tags: vec!["nginx:17".into()],
                                ..Default::default()
                            }],
                        }
                        .encode_to_vec(),
                        other => panic!("unexpected command: {other:?}"),
                    };
                    session.output(id, bytes);
                    session.complete(id, true);
                }
            }
        );
        if phase > 0 {
            let progress = result.as_array().unwrap();
            assert!(
                progress.iter().all(|item| item["errorMessage"].is_null()),
                "{result}"
            );
            assert_eq!(progress.last().unwrap()["dockerImageId"], image);
        }
        let inventory = f
            .lookup_state
            .platforms
            .platforms
            .list_images(f.platform_id)
            .await
            .unwrap();
        let summaries = f
            .lookup_state
            .platforms
            .image_updates
            .summaries(f.administrator.actor_id, true, f.platform_id, &inventory)
            .await
            .unwrap();
        let index = inventory
            .iter()
            .position(|item| item.docker_image_id == image)
            .unwrap();
        let update = &summaries[index][0];
        assert_eq!(
            update.status,
            [
                ImageUpdateStatus::UpdateAvailable,
                ImageUpdateStatus::UpToDate,
                ImageUpdateStatus::Unknown
            ][phase]
        );
        assert!(update.last_attempt_at > last_attempt);
        last_attempt = update.last_attempt_at;
        if phase == 1 {
            assert_eq!(update.remote_config_digest.as_ref(), Some(&image));
        }
    }
    assert!(commands.try_recv().is_err());
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn containerd_image_checks_compare_platform_content_and_fence_missing_identities() {
    use citadel_contracts::citadel::images::v1::{
        DistributionInspectRequest, DistributionInspectResponse, OciDescriptorResponse,
    };
    use citadel_platforms::image_updates::ImageUpdateStatus;
    use citadel_platforms::image_usage::ImageLocation;
    let f = fixture().await;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent',platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id).execute(&f.pool).await.unwrap();
    let image = format!("sha256:{}", "a".repeat(64));
    let config = format!("sha256:{}", "b".repeat(64));
    let manifest = format!("sha256:{}", "c".repeat(64));
    let index = format!("sha256:{}", "d".repeat(64));
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:17\"]')")
        .bind(Uuid::now_v7()).bind(&image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let (session, mut commands) = f
        .edge
        .register(EdgeTarget::platform(f.platform_id), Uuid::now_v7())
        .unwrap();
    let cancel = CancellationToken::new();
    let location = ImageLocation {
        docker_image_id: image.clone(),
        docker_node_id: None,
    };
    for (case, count, expected) in [
        ("same index", 3, ImageUpdateStatus::UpToDate),
        ("same manifest", 3, ImageUpdateStatus::UpToDate),
        ("other architecture changed", 4, ImageUpdateStatus::UpToDate),
        ("host config changed", 4, ImageUpdateStatus::UpdateAvailable),
        ("unconfirmed local content", 3, ImageUpdateStatus::Unknown),
        ("old agent", 2, ImageUpdateStatus::Unknown),
    ] {
        // Each case models independent immutable registry content.
        let runtime =
            citadel_adapters::connectors::routing::platforms::runtime::PlatformRuntimeRouter::new(
                f.pool.clone(),
                f.docker.clone(),
                f.agent.clone(),
                f.edge.clone(),
            );
        let service = citadel_platforms::image_updates::ImageUpdateService::new(
            Arc::new(runtime.clone()),
            Arc::new(citadel_adapters::connectors::registries::image_updates::ImageUpdateAdapter {
                pool: f.pool.clone(), runtime, cache: Arc::new(Default::default()),
            }),
            Arc::new(citadel_adapters::persistence::postgres::platforms::image_updates::PostgresImageUpdateStore(f.pool.clone())),
        );
        let (result, ()) = tokio::join!(
            service.check(
                f.administrator.actor_id,
                true,
                ImageUpdateTarget {
                    platform: f.platform_id,
                    location: &location,
                    reference: "nginx:17",
                },
                Uuid::from_u128(0x100),
                &cancel,
            ),
            async {
                for step in 0..count {
                    let envelope =
                        tokio::time::timeout(StdDuration::from_secs(10), commands.recv())
                            .await
                            .unwrap()
                            .unwrap();
                    let Some(core_envelope::Body::Command(command)) = envelope.body else {
                        panic!("image command")
                    };
                    let id = Uuid::parse_str(&command.command_id).unwrap();
                    let bytes = match EdgeCommandKind::try_from(command.kind).unwrap() {
                        EdgeCommandKind::ImageInspect => {
                            assert!(step == 0 || step == count - 1);
                            InspectImageResponse {
                                id: image.clone(),
                                content_digest: (case != "old agent").then(|| image.clone()),
                                repo_tags: vec!["nginx:17".into()],
                                os: Some("linux".into()),
                                architecture: Some("amd64".into()),
                                ..Default::default()
                            }
                            .encode_to_vec()
                        }
                        EdgeCommandKind::ImageDistributionInspect => {
                            let request =
                                DistributionInspectRequest::decode(command.payload.as_slice())
                                    .unwrap();
                            assert_eq!(request.target_platform.unwrap().architecture, "amd64");
                            let pinned = step == 2;
                            assert_eq!(
                                request.image_name,
                                if pinned {
                                    format!("docker.io/library/nginx@{image}")
                                } else {
                                    "nginx:17".into()
                                }
                            );
                            DistributionInspectResponse {
                                config_digest: if pinned && case == "host config changed" {
                                    manifest.clone()
                                } else {
                                    config.clone()
                                },
                                manifest_digest: if case == "same manifest" {
                                    image.clone()
                                } else {
                                    manifest.clone()
                                },
                                descriptor: Some(OciDescriptorResponse {
                                    digest: if case == "same index"
                                        || (pinned && case != "unconfirmed local content")
                                    {
                                        image.clone()
                                    } else {
                                        index.clone()
                                    },
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                            .encode_to_vec()
                        }
                        other => panic!("metadata check dispatched mutation: {other:?}"),
                    };
                    session.output(id, bytes);
                    session.complete(id, true);
                }
            }
        );
        let result = result.unwrap();
        assert_eq!(result.status, expected, "{case}");
        if expected == ImageUpdateStatus::UpToDate {
            assert_eq!(result.local_config_digest.as_ref(), Some(&config));
            assert_ne!(result.local_config_digest.as_ref(), Some(&image));
        }
        assert!(commands.try_recv().is_err());
    }
}
