use super::*;
use citadel_adapters::connectors::docker::LocalDockerSampler;
use citadel_platforms::PlatformHealthPort;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn health_is_ping_only_and_sampling_discovers_once_with_slow_metadata() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let requests = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let observed = requests.clone();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = tokio::select! { ()=shutdown.cancelled()=>break, value=listener.accept()=>value.unwrap() };
            let mut bytes = Vec::new();
            let mut buffer = [0; 2048];
            while !bytes.windows(4).any(|value| value == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 8192);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8_lossy(&bytes);
            let path = request
                .split_whitespace()
                .nth(1)
                .unwrap()
                .split('?')
                .next()
                .unwrap();
            *observed.lock().unwrap().entry(path.into()).or_default() += 1;
            let body = match path {
                "/_ping" => "OK",
                "/version" => r#"{"Version":"27","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                "/v1.49/info" => r#"{"ID":"daemon","NCPU":2,"MemTotal":1024}"#,
                "/v1.49/containers/json" => {
                    r#"[{"Id":"running","State":"running"},{"Id":"stopped","State":"exited"}]"#
                }
                "/v1.49/containers/running/stats" => r#"{"read":"2026-09-19T00:00:00Z"}"#,
                "/v1.49/volumes" => r#"{"Volumes":[]}"#,
                "/v1.49/system/df" => r#"{"LayersSize":0,"Volumes":[]}"#,
                "/v1.49/images/json" if observed.lock().unwrap()[path] > 1 => {
                    r#"[{"Id":"new-image"}]"#
                }
                "/v1.49/images/json" | "/v1.49/networks" => "[]",
                other => panic!("unexpected Docker operation: {other}"),
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
    docker.probe(&stop).await.unwrap();
    assert_eq!(
        *requests.lock().unwrap(),
        HashMap::from([("/_ping".into(), 1)])
    );
    let mut sampler = LocalDockerSampler::new(docker.clone(), 2);
    for _ in 0..2 {
        let sample = sampler.sample(&stop).await.unwrap();
        assert_eq!(sample.containers.stats.len(), 1);
        assert_eq!(sample.containers.failed_samples, 0);
        let platform = sample.platform.unwrap();
        assert_eq!(
            (
                platform.container_count,
                platform.containers_running,
                platform.containers_stopped
            ),
            (2, 1, 1)
        );
    }
    {
        let calls = requests.lock().unwrap();
        assert_eq!(calls["/v1.49/containers/json"], 2);
        assert_eq!(calls["/v1.49/containers/running/stats"], 2);
        for path in [
            "/version",
            "/v1.49/info",
            "/v1.49/images/json",
            "/v1.49/networks",
            "/v1.49/volumes",
            "/v1.49/system/df",
        ] {
            assert_eq!(calls[path], 1, "{path}");
        }
    }
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(61)).await;
    tokio::time::resume();
    let refreshed = sampler.sample(&stop).await.unwrap();
    assert_eq!(
        refreshed.platform.unwrap().image_count,
        1,
        "new inventory counts converge on the one-minute refresh"
    );
    assert_eq!(requests.lock().unwrap()["/v1.49/containers/json"], 3);
    docker.invalidate_daemon().await;
    sampler.sample(&stop).await.unwrap();
    {
        let calls = requests.lock().unwrap();
        assert_eq!(calls["/version"], 2, "reconnect must renegotiate");
        assert_eq!(
            calls["/v1.49/images/json"], 3,
            "reconnect invalidates slow metadata"
        );
        assert_eq!(calls["/v1.49/containers/json"], 4);
    }
    stop.cancel();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
