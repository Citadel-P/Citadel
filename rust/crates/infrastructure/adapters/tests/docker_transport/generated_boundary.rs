use super::*;
use citadel_adapters::connectors::docker::DockerError;

#[tokio::test]
async fn generated_calls_share_a_connection_and_renegotiate_after_bounded_400() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        // First three calls reuse one connection across two generated API tags.
        let (mut socket, _) = listener.accept().await.unwrap();
        for (expected, body) in [
            (
                "GET /version ",
                r#"{"ApiVersion":"1.45","MinAPIVersion":"1.41"}"#,
            ),
            ("GET /v1.45/info ", r#"{"ID":"shared"}"#),
            ("GET /v1.45/networks ", "[]"),
        ] {
            let (request, _) = read_terminal_request(&mut socket).await;
            assert!(request.starts_with(expected), "{request}");
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /v1.45/info "));
        socket
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 65537\r\n\r\n")
            .await
            .unwrap();
        drop(socket);
        let (mut socket, _) = listener.accept().await.unwrap();
        for (expected, body) in [
            (
                "GET /version ",
                r#"{"ApiVersion":"1.44","MinAPIVersion":"1.41"}"#,
            ),
            ("GET /v1.44/info ", r#"{"ID":"renegotiated"}"#),
        ] {
            let (request, _) = read_terminal_request(&mut socket).await;
            assert!(request.starts_with(expected), "{request}");
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    assert_eq!(client.info().await.unwrap().id, "shared");
    assert!(client.list_networks().await.unwrap().is_empty());
    assert!(matches!(
        client.info().await,
        Err(DockerError::ResponseTooLarge { limit: 65536 })
    ));
    assert_eq!(client.clone().info().await.unwrap().id, "renegotiated");
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn finite_success_bodies_are_bounded_with_or_without_content_length() {
    for chunked in [false, true] {
        let path = temp_socket();
        let listener = UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_terminal_request(&mut socket).await;
            let body = r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            read_terminal_request(&mut socket).await;
            if chunked {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                    .await
                    .unwrap();
                let block = vec![b' '; 64 * 1024];
                for _ in 0..257 {
                    if socket.write_all(b"10000\r\n").await.is_err() {
                        break;
                    }
                    if socket.write_all(&block).await.is_err() {
                        break;
                    }
                    if socket.write_all(b"\r\n").await.is_err() {
                        break;
                    }
                }
            } else {
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16777217\r\n\r\n")
                    .await
                    .unwrap();
            }
        });
        let client = DockerClient::new(&path, Duration::from_secs(5)).unwrap();
        assert!(matches!(
            client.info().await,
            Err(DockerError::ResponseTooLarge { limit: 16777216 })
        ));
        server.await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn swarm_update_preserves_unknown_nested_fields_and_64_bit_version() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let spec = serde_json::json!({"Name":"future","FutureSpec":{"enabled":true},"TaskTemplate":{"ContainerSpec":{"Image":"alpine","FutureContainer":{"limit":42}},"FutureTask":7}});
    let expected = spec.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        for body in [r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#.to_owned(), serde_json::json!({"ID":"service","Version":{"Index":5_000_000_000_i64},"Spec":expected}).to_string()] {
            read_terminal_request(&mut socket).await;
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
        let (headers, body) = read_terminal_request(&mut socket).await;
        assert!(headers.starts_with("POST /v1.49/services/service/update?version=5000000000 "));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            expected
        );
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            .await
            .unwrap();
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let service = client.inspect_swarm_service("service").await.unwrap();
    assert_eq!(service.spec, spec);
    client
        .update_swarm_service("service", service.version.index as i64, &service.spec)
        .await
        .unwrap();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn generated_request_timeout_is_classified_as_transport_timeout() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_terminal_request(&mut socket).await;
        let body = r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        read_terminal_request(&mut socket).await;
        tokio::time::sleep(Duration::from_millis(150)).await;
    });
    let client = DockerClient::new(&path, Duration::from_millis(50)).unwrap();
    assert!(matches!(client.info().await,Err(DockerError::Transport(error)) if error.is_timeout()));
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
