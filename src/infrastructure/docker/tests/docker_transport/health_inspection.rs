use super::*;
use citadel_platforms::ContainerInventoryPort;
use serde_json::json;
use tokio_util::sync::CancellationToken;

async fn reply(socket: &mut tokio::net::UnixStream, body: &str) {
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
async fn narrow_health_keeps_full_inspection_available_and_rejects_mismatched_identity() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let document = json!({"Id":"fixture","RestartCount":2,
        "Config":{"Env":["TEST_SECRET=fixture-only"],"Labels":{"example":"keep"},"Healthcheck":{"Test":["CMD","true"]}},
        "State":{"Status":"running","OOMKilled":false,"ExitCode":0,"Health":{"Status":"healthy","FailingStreak":0,"Log":[{"End":"2026-11-16T10:00:00Z","Output":"keep in full inspection"}]}},
        "NetworkSettings":{"Networks":{"example":{"IPAddress":"172.19.0.2"}}},
        "FutureDaemonField":{"Nested":[1,2,3]}});
    let expected = document.clone();
    let server = tokio::spawn(async move {
        for index in 0..5 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (request, _) = read_terminal_request(&mut socket).await;
            if index == 0 {
                assert!(request.starts_with("GET /version "));
                reply(
                    &mut socket,
                    r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                )
                .await;
            } else if index == 3 {
                assert!(request.contains("/containers/json?"));
                reply(&mut socket, r#"[{"Id":"fixture","State":"exited"}]"#).await;
            } else {
                assert!(request.starts_with("GET /v1.49/containers/fixture/json "));
                let mut doc = document.clone();
                if index == 4 {
                    doc["Id"] = json!("different-container");
                }
                reply(&mut socket, &doc.to_string()).await;
            }
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let narrow = client.inspect_container_health("fixture").await.unwrap();
    assert_eq!(narrow.id.as_deref(), Some("fixture"));
    assert_eq!(narrow.state, "running");
    assert_eq!(narrow.health.restart_count, Some(2));
    assert_eq!(
        client.inspect_container_document("fixture").await.unwrap(),
        expected
    );
    let cancel = CancellationToken::new();
    let observed = client.observe_containers(&cancel).await.unwrap();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].state, "exited");
    assert!(observed[0].health.is_none());
    cancel.cancel();
    assert!(client.observe_containers(&cancel).await.is_err());
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn health_inspection_preserves_transport_errors_and_response_limit() {
    use citadel_docker::client::DockerError;
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for index in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_terminal_request(&mut socket).await;
            match index {
                0 => {
                    reply(
                        &mut socket,
                        r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                    )
                    .await
                }
                1 => {
                    socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
                }
                2 => reply(&mut socket, "{invalid").await,
                _ => {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16777217\r\nConnection: close\r\n\r\n").await.unwrap();
                }
            }
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    assert!(matches!(
        client.inspect_container_health(" ").await,
        Err(DockerError::InvalidIdentifier)
    ));
    assert!(
        matches!(client.inspect_container_health("fixture").await,Err(DockerError::Api { status, .. }) if status == reqwest::StatusCode::NOT_FOUND)
    );
    assert!(matches!(
        client.inspect_container_health("fixture").await,
        Err(DockerError::InvalidJson(_))
    ));
    assert!(matches!(
        client.inspect_container_health("fixture").await,
        Err(DockerError::ResponseTooLarge { .. })
    ));
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn collection_cancels_an_inflight_health_response() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        for body in [
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            r#"[{"Id":"fixture","State":"running"}]"#,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_terminal_request(&mut socket).await;
            reply(&mut socket, body).await;
        }
        let (mut socket, _) = listener.accept().await.unwrap();
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /v1.49/containers/fixture/json "));
        started.send(()).unwrap();
        stop.cancelled().await;
    });
    let client = DockerClient::new(&path, Duration::from_secs(30)).unwrap();
    let token = cancel.clone();
    let collecting = tokio::spawn(async move { client.observe_containers(&token).await });
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    cancel.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), collecting)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
