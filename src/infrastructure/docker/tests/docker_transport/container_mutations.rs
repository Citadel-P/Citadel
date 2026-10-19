use super::*;
use citadel_platforms::containers::{ContainerAction, DeleteContainerOptions};

#[tokio::test]
async fn container_actions_use_generated_routes_preserve_delete_options_and_accept_304() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for expected in [
            "GET /version ",
            "POST /v1.49/containers/container/start ",
            "POST /v1.49/containers/container/stop?signal=SIGTERM&t=10 ",
            "POST /v1.49/containers/container/restart?signal=SIGINT&t=5 ",
            "POST /v1.49/containers/container/pause ",
            "POST /v1.49/containers/container/unpause ",
            "DELETE /v1.49/containers/container?v=false&force=true&link=false ",
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 2048];
            while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 8192);
                bytes.extend_from_slice(&buffer[..n]);
            }
            assert!(
                String::from_utf8_lossy(&bytes).starts_with(expected),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
            let (status, body) = if expected.starts_with("GET") {
                ("200 OK", r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#)
            } else if expected.contains("/start ") || expected.contains("/stop?") {
                ("304 Not Modified", "")
            } else {
                ("204 No Content", "")
            };
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    for action in [
        ContainerAction::Start,
        ContainerAction::Stop,
        ContainerAction::Restart,
        ContainerAction::Pause,
        ContainerAction::Unpause,
        ContainerAction::Delete(DeleteContainerOptions {
            force: true,
            v: false,
            link: false,
        }),
    ] {
        client
            .change_container_state("container", action)
            .await
            .unwrap();
    }
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
