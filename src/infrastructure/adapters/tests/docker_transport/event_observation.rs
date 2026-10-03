use super::*;

async fn respond(socket: &mut tokio::net::UnixStream, body: &str) {
    socket
        .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes())
        .await
        .unwrap();
}

fn assert_filter(request: &str, id: &str) {
    let path = request.split_whitespace().nth(1).unwrap();
    let url = url::Url::parse(&format!("http://docker{path}")).unwrap();
    assert_eq!(url.path(), "/v1.49/containers/json");
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query.get("all").map(String::as_str), Some("true"));
    let filter: serde_json::Value = serde_json::from_str(&query["filters"]).unwrap();
    assert_eq!(filter, serde_json::json!({"id": [id]}));
    assert!(!query.contains_key("size"));
}

#[tokio::test]
async fn repeated_six_container_changes_only_read_filtered_summaries() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /version "));
        respond(
            &mut socket,
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
        )
        .await;
        for cycle in 0..6 {
            for index in 0..6 {
                let id = format!("container-{index}");
                let (request, _) = read_terminal_request(&mut socket).await;
                assert_filter(&request, &id);
                respond(&mut socket, &serde_json::json!([{
                    "Id":id, "Names":[format!("/web-{index}")], "Image":"alpine", "ImageID":"sha256:image",
                    "State":if cycle % 2 == 0 {"running"} else {"exited"}, "Created":123,
                    "Labels":{"com.docker.compose.project":"demo", "com.citadel.system":"true", "com.citadel.system-role":"core"},
                    "Ports":[{"IP":"0.0.0.0", "PrivatePort":80, "PublicPort":8080, "Type":"tcp"}]
                }]).to_string()).await;
            }
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    for cycle in 0..6 {
        for index in 0..6 {
            let id = format!("container-{index}");
            let observed = client
                .container_event_observation(&id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(observed.id, id);
            assert_eq!(observed.name, format!("web-{index}"));
            assert_eq!(
                observed.state,
                if cycle % 2 == 0 { "running" } else { "exited" }
            );
            assert_eq!(observed.image_id, "sha256:image");
            assert_eq!(observed.stack.as_deref(), Some("demo"));
            assert!(observed.is_system);
            assert!(observed.has_citadel_ownership_labels);
            assert_eq!(observed.system_role.as_deref(), Some("core"));
            assert_eq!(observed.ports["80/tcp"][0]["hostPort"], "8080");
        }
    }
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn incomplete_event_summary_falls_back_and_missing_identity_is_not_a_tombstone() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /version "));
        respond(
            &mut socket,
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
        )
        .await;
        let (request, _) = read_terminal_request(&mut socket).await;
        assert_filter(&request, "fixture");
        respond(&mut socket, r#"[{"Id":"fixture","Image":"alpine"}]"#).await;
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /v1.49/containers/fixture/json "));
        respond(&mut socket, r#"{"Id":"fixture","Name":"/web","Image":"sha256:image","Created":"2026-01-01T00:00:00Z","State":{"Status":"running"},"Config":{"Image":"alpine","Labels":{"com.docker.swarm.task.id":"task"}},"NetworkSettings":{"Ports":{"80/tcp":[{"HostIp":"0.0.0.0","HostPort":"8080"}],"90/tcp":null}}}"#).await;
        for body in ["[]", r#"[{"Id":"fixture-other","ImageID":"wrong"}]"#] {
            let (request, _) = read_terminal_request(&mut socket).await;
            assert_filter(&request, "fixture");
            respond(&mut socket, body).await;
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let observed = client
        .container_event_observation("fixture")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed.image_id, "sha256:image");
    assert_eq!(observed.name, "web");
    assert!(observed.is_swarm_task);
    assert_eq!(observed.ports["80/tcp"][0]["hostPort"], "8080");
    assert_eq!(observed.ports["90/tcp"], serde_json::json!([]));
    for _ in 0..2 {
        assert!(
            client
                .container_event_observation("fixture")
                .await
                .unwrap()
                .is_none()
        );
    }
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn network_list_recovers_usage_from_one_container_discovery() {
    use citadel_platforms::NetworkInventoryPort;
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let body = if request.starts_with("GET /version ") {
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else if request.starts_with("GET /v1.49/networks ") {
                r#"[{"Id":"attached","Name":"used"},{"Id":"unused","Name":"empty"}]"#
            } else {
                assert!(
                    request.starts_with("GET /v1.49/containers/json?all=true "),
                    "{request}"
                );
                r#"[{"Id":"web","NetworkSettings":{"Networks":{"used":{"NetworkID":"attached"}}}}]"#
            };
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
    });
    let docker = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let result =
        NetworkInventoryPort::list_networks(&docker, &tokio_util::sync::CancellationToken::new())
            .await
            .unwrap();
    assert_eq!(result[0].container_count, 1);
    assert_eq!(result[1].container_count, 0);
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn lifecycle_observations_inspect_one_resource_and_tombstones_perform_no_io() {
    use citadel_platforms::jobs::{ResourceChange as C, ResourceDelta as D, RuntimeEventKind as K};
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..6 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let body = if request.starts_with("GET /version ") {
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else if request.starts_with("GET /v1.49/networks/net ") {
                r#"{"Id":"net","Name":"bridge"}"#
            } else if request.starts_with("GET /v1.49/volumes/data ") {
                r#"{"Name":"data"}"#
            } else if request.starts_with("GET /v1.49/system/df?type=volume ") {
                r#"{"Volumes":[]}"#
            } else if request.starts_with("GET /v1.49/images/img/json ") {
                r#"{"Id":"img","RepoTags":["example:v1"],"RepoDigests":["example@sha256:abc"],"Created":"2026-09-27T00:00:00Z","Size":123}"#
            } else {
                assert!(
                    request.starts_with("GET /v1.49/containers/json?all=true&filters="),
                    "{request}"
                );
                assert!(
                    urlencoding::decode(&request)
                        .unwrap()
                        .contains(r#""ancestor":["img"]"#)
                );
                r#"[{"Id":"container","ImageID":"img"}]"#
            };
            respond(&mut socket, body).await;
        }
    });
    let docker = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    for kind in [
        K::Image(C::Tombstone),
        K::Network(C::Tombstone),
        K::Volume(C::Tombstone),
    ] {
        assert!(
            docker
                .resource_event_observation(kind, "gone", &cancel)
                .await
                .unwrap()
                .unwrap()
                .valid_for(kind)
        );
    }
    assert!(matches!(
        docker
            .resource_event_observation(K::Network(C::Observe), "net", &cancel)
            .await
            .unwrap(),
        Some(D::Network { value: Some(_), .. })
    ));
    assert!(matches!(
        docker
            .resource_event_observation(K::Volume(C::Observe), "data", &cancel)
            .await
            .unwrap(),
        Some(D::Volume { value: Some(_), .. })
    ));
    let Some(D::Image {
        value: Some(image), ..
    }) = docker
        .resource_event_observation(K::Image(C::Observe), "img", &cancel)
        .await
        .unwrap()
    else {
        panic!("image expected")
    };
    assert_eq!(image.containers, 1);
    assert_eq!(image.repo_tags, ["example:v1"]);
    assert_eq!(image.repo_digests, ["example@sha256:abc"]);
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
