use super::*;
use citadel_platforms::ImageInventoryPort;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn image_inventory_counts_running_and_stopped_references_without_per_image_reads() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let phase = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let stop = CancellationToken::new();
    let server = tokio::spawn({
        let phase = phase.clone();
        let calls = calls.clone();
        let stop = stop.clone();
        async move {
            loop {
                let (mut socket, _) = tokio::select! { ()=stop.cancelled()=>break, next=listener.accept()=>next.unwrap() };
                let mut bytes = Vec::new();
                let mut buffer = [0; 2048];
                while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                    let n = socket.read(&mut buffer).await.unwrap();
                    assert!(n > 0 && bytes.len() < 8192);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8_lossy(&bytes);
                let route = request.split_whitespace().nth(1).unwrap();
                calls.lock().unwrap().push(route.to_owned());
                let mut status = "200 OK";
                let body = match route {
                    "/version" => r#"{"Version":"27","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                    "/v1.49/images/json?all=true" => {
                        r#"[{"Id":"sha256:used","Containers":-1},{"Id":"sha256:stopped-only","Containers":-1},{"Id":"sha256:unused","Containers":99}]"#
                    }
                    "/v1.49/containers/json?all=true" => match phase.load(Ordering::Acquire) {
                        0 => {
                            r#"[{"Id":"running","ImageID":"sha256:used","State":"running"},{"Id":"stopped","ImageID":"sha256:used","State":"exited"},{"Id":"created","ImageID":"sha256:stopped-only","State":"created"},{"Id":"unresolved","Image":"sha256:unused","State":"running"}]"#
                        }
                        1 => "[]",
                        _ => {
                            status = "500 Internal Server Error";
                            r#"{"message":"container list unavailable"}"#
                        }
                    },
                    other => panic!("Unexpected Docker call: {other}"),
                };
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        }
    });
    let docker = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let images = ImageInventoryPort::list_images(&docker, &stop)
        .await
        .unwrap();
    assert_eq!(
        images
            .iter()
            .map(|i| (i.id.as_str(), i.containers))
            .collect::<Vec<_>>(),
        vec![
            ("sha256:used", 2),
            ("sha256:stopped-only", 1),
            ("sha256:unused", 0)
        ]
    );
    phase.store(1, Ordering::Release);
    assert!(
        ImageInventoryPort::list_images(&docker, &stop)
            .await
            .unwrap()
            .iter()
            .all(|i| i.containers == 0),
        "removing the last container clears usage"
    );
    phase.store(2, Ordering::Release);
    assert!(
        ImageInventoryPort::list_images(&docker, &stop)
            .await
            .is_err(),
        "failed discovery must not overwrite usage with false zeroes"
    );
    // Slow platform metadata only needs the image count and keeps the raw list path.
    assert_eq!(docker.list_images().await.unwrap().len(), 3);
    let calls = calls.lock().unwrap().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|p| p.starts_with("/v1.49/containers/json"))
            .count(),
        3
    );
    assert_eq!(
        calls
            .iter()
            .filter(|p| p.starts_with("/v1.49/images/json"))
            .count(),
        4
    );
    stop.cancel();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
