use super::*;

#[tokio::test]
async fn distribution_uses_scoped_registry_auth_and_does_not_echo_upstream_errors() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for step in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            if step == 0 {
                assert!(request.starts_with("GET /version "));
            } else {
                assert!(
                    request.starts_with(
                        "GET /v1.49/distribution/registry.test%3A5000%2Fapp%3Alatest/json "
                    ),
                    "{request}"
                );
            }
            let auth = request
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("x-registry-auth:"));
            if step < 2 {
                assert!(auth.is_none());
            } else {
                assert_eq!(
                    auth.unwrap().split_once(':').unwrap().1.trim(),
                    "disposable-registry-auth"
                );
            }
            let body = if step == 0 {
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else if step == 3 {
                "upstream echoed disposable-registry-auth"
            } else {
                r#"{"Descriptor":{"digest":"sha256:abc"}}"#
            };
            let status = if step == 3 {
                "401 Unauthorized"
            } else {
                "200 OK"
            };
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let image = "registry.test:5000/app:latest";
    client.distribution_inspect(image).await.unwrap();
    client
        .distribution_inspect_authenticated(image, Some("disposable-registry-auth"))
        .await
        .unwrap();
    let error = client
        .distribution_inspect_authenticated(image, Some("disposable-registry-auth"))
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("disposable-registry-auth"));
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
