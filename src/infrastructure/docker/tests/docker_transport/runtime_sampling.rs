use super::*;
use citadel_docker::client::LocalDockerSampler;
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
                    assert!(request.contains("all=true"));
                    assert!(!request.contains("size=true"));
                    r#"[{"Id":"running","State":"running","Names":["/application"],"Labels":{"keep":"inventory"},"Mounts":[{"Type":"volume","Name":"data"}],"FutureDaemonField":{"nested":[1,2,3]}},{"Id":"stopped","State":"exited"},{"Id":"paused","State":"paused"},{"State":"created"},{"State":"restarting"},{"State":"removing"},{"State":"dead"},{"State":null},{}]"#
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
        let mut sample = sampler.sample(&stop).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while sample.platform.is_none() {
                sampler.enrich_metadata(&mut sample);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(sample.containers.stats.len(), 1);
        assert_eq!(sample.containers.failed_samples, 0);
        let platform = sample.platform.unwrap();
        assert_eq!(
            (
                platform.container_count,
                platform.containers_running,
                platform.containers_stopped,
                platform.containers_paused
            ),
            (9, 1, 1, 1)
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
    let mut refreshed = sampler.sample(&stop).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while refreshed
            .platform
            .as_ref()
            .is_none_or(|v| v.image_count != 1)
        {
            sampler.enrich_metadata(&mut refreshed);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        refreshed.platform.unwrap().image_count,
        1,
        "new inventory counts converge on the one-minute refresh"
    );
    assert_eq!(requests.lock().unwrap()["/v1.49/containers/json"], 3);
    docker.invalidate_daemon().await;
    let mut sample = sampler.sample(&stop).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while sample.platform.is_none() {
            sampler.enrich_metadata(&mut sample);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    {
        let calls = requests.lock().unwrap();
        assert_eq!(calls["/version"], 2, "reconnect must renegotiate");
        assert_eq!(
            calls["/v1.49/images/json"], 3,
            "reconnect invalidates slow metadata"
        );
        assert_eq!(calls["/v1.49/containers/json"], 4);
    }
    // The same daemon document remains available to full inventory consumers.
    let inventory = docker
        .list_container_models(Some(true), None, None, None)
        .await
        .unwrap();
    assert_eq!(inventory[0].names.as_ref().unwrap(), &["/application"]);
    assert_eq!(inventory[0].labels.as_ref().unwrap()["keep"], "inventory");
    assert_eq!(
        inventory[0].mounts.as_ref().unwrap()[0].name.as_deref(),
        Some("data")
    );
    assert_eq!(
        inventory[0].additional_properties["FutureDaemonField"]["nested"][2],
        3
    );
    stop.cancel();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn sampling_discovery_bounds_responses_and_preserves_daemon_errors() {
    for (response, expected) in [
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 16777217\r\nConnection: close\r\n\r\n",
            "exceeded",
        ),
        (
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 7\r\nConnection: close\r\n\r\noffline",
            "offline",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\n[",
            "invalid JSON",
        ),
    ] {
        let path = temp_socket();
        let listener = UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (request, _) = read_terminal_request(&mut socket).await;
            assert!(request.starts_with("GET /version "));
            let body = r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#;
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
            let (mut socket, _) = listener.accept().await.unwrap();
            let (request, _) = read_terminal_request(&mut socket).await;
            assert!(request.starts_with("GET /v1.49/containers/json?all=true "));
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let docker = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
        let mut sampler = LocalDockerSampler::new(docker, 2);
        let error = sampler
            .sample(&CancellationToken::new())
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains(expected), "{error}");
        server.await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn sampling_cancels_inflight_discovery() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = read_terminal_request(&mut socket).await;
        let body = r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#;
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
        let (mut socket, _) = listener.accept().await.unwrap();
        let (request, _) = read_terminal_request(&mut socket).await;
        assert!(request.starts_with("GET /v1.49/containers/json?all=true "));
        started.send(()).unwrap();
        stop.cancelled().await;
    });
    let docker = DockerClient::new(&path, Duration::from_secs(30)).unwrap();
    let token = cancel.clone();
    let collecting =
        tokio::spawn(async move { LocalDockerSampler::new(docker, 2).sample(&token).await });
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    cancel.cancel();
    let result = tokio::time::timeout(Duration::from_secs(2), collecting)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(result, Err(error) if error.kind == citadel_platforms::RuntimeErrorKind::Cancelled)
    );
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
