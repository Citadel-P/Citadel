#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use uuid::Uuid;

#[tokio::test]
async fn generated_subset_uses_versioned_unix_socket_requests_and_bounded_streams() {
    let socket_path = temp_socket();
    let listener = UnixListener::bind(&socket_path).unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for _ in 0..7 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = socket.read(&mut chunk).await.unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                assert!(
                    request.len() < 16 * 1024,
                    "fixture request header grew unexpectedly"
                );
            }
            let first_line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap()
                .to_owned();
            let path = first_line.split_whitespace().nth(1).unwrap().to_owned();
            let body = response_for(&path);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(header.as_bytes()).await.unwrap();
            if path == "/v1.49/events" {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let split = body.len() / 2;
            socket.write_all(&body[..split]).await.unwrap();
            socket.write_all(&body[split..]).await.unwrap();
            socket.shutdown().await.unwrap();
            requests.push(path);
        }
        requests
    });

    let client = DockerClient::new(&socket_path, Duration::from_millis(50)).unwrap();
    client.ping().await.unwrap();
    assert_eq!(
        client.negotiated_version().await.unwrap().to_string(),
        "1.49"
    );
    assert_eq!(client.info().await.unwrap().id, "daemon-fixture");
    let containers = client.list_containers(true).await.unwrap();
    assert_eq!(containers[0].id, "container-fixture");
    assert_eq!(
        client
            .inspect_container("container-fixture")
            .await
            .unwrap()
            .name,
        "/fixture"
    );
    let mut events = client.events(None, None).await.unwrap();
    assert_eq!(events.next().await.unwrap().unwrap().action, "start");
    drop(events);
    let mut stats = client.container_stats("container-fixture").await.unwrap();
    assert_eq!(
        stats.next().await.unwrap().unwrap().memory_stats.usage,
        4096
    );
    drop(stats);

    let requests = server.await.unwrap();
    assert_eq!(
        requests,
        vec![
            "/_ping",
            "/version",
            "/v1.49/info",
            "/v1.49/containers/json?all=true",
            "/v1.49/containers/container-fixture/json",
            "/v1.49/events",
            "/v1.49/containers/container-fixture/stats?stream=true",
        ]
    );
    std::fs::remove_file(socket_path).unwrap();
}

fn response_for(path: &str) -> Vec<u8> {
    let body = match path {
        "/_ping" => "OK",
        "/version" => {
            r#"{"Version":"29.6.2","ApiVersion":"1.53","MinAPIVersion":"1.44","GitCommit":"fixture","Os":"linux","Arch":"amd64"}"#
        }
        "/v1.49/info" => {
            r#"{"ID":"daemon-fixture","Containers":1,"ContainersRunning":1,"ContainersStopped":0,"ContainersPaused":0,"Images":1,"NCPU":2,"MemTotal":1073741824}"#
        }
        "/v1.49/containers/json?all=true" => {
            r#"[{"Id":"container-fixture","Names":["/fixture"],"Image":"nginx:alpine","ImageID":"sha256:fixture","Created":1,"Labels":{},"State":"running","Status":"Up"}]"#
        }
        "/v1.49/containers/container-fixture/json" => {
            r#"{"Id":"container-fixture","Created":"2026-01-01T00:00:00Z","Path":"nginx","Args":[],"State":{"Status":"running","Running":true},"Image":"sha256:fixture","Name":"/fixture","Config":{"Image":"nginx:alpine","Labels":{}}}"#
        }
        "/v1.49/events" => {
            "{\"Type\":\"container\",\"Action\":\"start\",\"Actor\":{\"ID\":\"container-fixture\",\"Attributes\":{}},\"scope\":\"local\",\"time\":1,\"timeNano\":1000000000}\n"
        }
        "/v1.49/containers/container-fixture/stats?stream=true" => {
            "{\"id\":\"container-fixture\",\"name\":\"fixture\",\"cpu_stats\":{\"cpu_usage\":{\"total_usage\":2},\"system_cpu_usage\":4,\"online_cpus\":2},\"precpu_stats\":{\"cpu_usage\":{\"total_usage\":1},\"system_cpu_usage\":2,\"online_cpus\":2},\"memory_stats\":{\"usage\":4096,\"limit\":8192},\"networks\":{}}\n"
        }
        unexpected => panic!("unexpected request {unexpected}"),
    };
    body.as_bytes().to_vec()
}

fn temp_socket() -> PathBuf {
    std::env::temp_dir().join(format!("citadel-phase0-{}.sock", Uuid::now_v7()))
}
