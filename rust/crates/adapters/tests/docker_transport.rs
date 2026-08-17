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
        for _ in 0..19 {
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
    let swarm = client.inspect_swarm().await.unwrap();
    assert_eq!(swarm.id, "swarm-fixture");
    assert_eq!(swarm.version.index, 7);
    assert_eq!(client.list_images().await.unwrap()[0].id, "sha256:image");
    assert_eq!(client.list_volumes().await.unwrap()[0].name, "data");
    assert_eq!(client.inspect_volume("data").await.unwrap().driver, "local");
    assert_eq!(client.list_networks().await.unwrap()[0].name, "bridge");
    assert_eq!(
        client
            .inspect_network("network-1")
            .await
            .unwrap()
            .peers
            .len(),
        1
    );
    assert_eq!(client.list_swarm_nodes().await.unwrap()[0].id, "node-1");
    assert_eq!(
        client.list_swarm_services().await.unwrap()[0].id,
        "service-1"
    );
    assert_eq!(client.list_swarm_tasks().await.unwrap()[0].id, "task-1");
    assert_eq!(client.list_swarm_secrets().await.unwrap()[0].id, "secret-1");
    assert_eq!(client.list_swarm_configs().await.unwrap()[0].id, "config-1");
    let mut events = client.events(None, None).await.unwrap();
    assert_eq!(events.next().await.unwrap().unwrap().action, "start");
    drop(events);
    let mut stats = client.container_stats("container-fixture").await.unwrap();
    assert_eq!(
        stats.next().await.unwrap().unwrap().memory_stats.usage,
        4096
    );
    drop(stats);
    assert_eq!(
        client
            .container_stats_once("container-fixture")
            .await
            .unwrap()
            .memory_stats
            .usage,
        4096
    );

    let requests = server.await.unwrap();
    assert_eq!(
        requests,
        vec![
            "/_ping",
            "/version",
            "/v1.49/info",
            "/v1.49/containers/json?all=true",
            "/v1.49/containers/container-fixture/json",
            "/v1.49/swarm",
            "/v1.49/images/json?all=true",
            "/v1.49/volumes",
            "/v1.49/volumes/data",
            "/v1.49/networks",
            "/v1.49/networks/network-1",
            "/v1.49/nodes",
            "/v1.49/services?status=true",
            "/v1.49/tasks?filters=%7B%22desired-state%22%3A%5B%22running%22%5D%7D",
            "/v1.49/secrets",
            "/v1.49/configs",
            "/v1.49/events",
            "/v1.49/containers/container-fixture/stats?stream=true",
            "/v1.49/containers/container-fixture/stats?stream=false&one-shot=true",
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
        "/v1.49/swarm" => {
            r#"{"ID":"swarm-fixture","Version":{"Index":7},"CreatedAt":"2026-01-01T00:00:00Z","UpdatedAt":"2026-01-02T00:00:00Z","JoinTokens":{"Worker":"worker-secret","Manager":"manager-secret"}}"#
        }
        "/v1.49/images/json?all=true" => {
            r#"[{"Id":"sha256:image","RepoTags":["nginx:alpine"],"RepoDigests":[],"Created":1,"Size":10,"Labels":{},"Containers":1}]"#
        }
        "/v1.49/volumes" => {
            r#"{"Volumes":[{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{}}],"Warnings":[]}"#
        }
        "/v1.49/volumes/data" => {
            r#"{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{}}"#
        }
        "/v1.49/networks" => {
            r#"[{"Name":"bridge","Id":"network-1","Created":"2026-01-01T00:00:00Z","Scope":"local","Driver":"bridge","EnableIPv4":true,"Labels":{},"Options":{}}]"#
        }
        "/v1.49/networks/network-1" => {
            r#"{"Name":"bridge","Id":"network-1","Created":"2026-01-01T00:00:00Z","Scope":"local","Driver":"bridge","EnableIPv4":true,"Containers":{},"Peers":[{"Name":"peer-1","IP":"10.0.0.2"}],"Labels":{},"Options":{}}"#
        }
        "/v1.49/nodes" => {
            r#"[{"ID":"node-1","Version":{"Index":1},"Spec":{"Availability":"active"},"Description":{"Hostname":"worker"},"Status":{"State":"ready"}}]"#
        }
        "/v1.49/services?status=true" => {
            r#"[{"ID":"service-1","Version":{"Index":1},"Spec":{"Name":"web","TaskTemplate":{"ContainerSpec":{"Image":"nginx:alpine"}},"Mode":{"Replicated":{"Replicas":1}}},"ServiceStatus":{"RunningTasks":1,"DesiredTasks":1}}]"#
        }
        "/v1.49/tasks?filters=%7B%22desired-state%22%3A%5B%22running%22%5D%7D" => {
            r#"[{"ID":"task-1","Version":{"Index":1},"Name":"web.1","ServiceID":"service-1","NodeID":"node-1","DesiredState":"running","Status":{"State":"running"},"Spec":{"ContainerSpec":{"Image":"nginx:alpine"}}}]"#
        }
        "/v1.49/secrets" => {
            r#"[{"ID":"secret-1","Version":{"Index":1},"Spec":{"Name":"password","Labels":{}}}]"#
        }
        "/v1.49/configs" => {
            r#"[{"ID":"config-1","Version":{"Index":1},"Spec":{"Name":"app-config","Labels":{}}}]"#
        }
        "/v1.49/events" => {
            "{\"Type\":\"container\",\"Action\":\"start\",\"Actor\":{\"ID\":\"container-fixture\",\"Attributes\":{}},\"scope\":\"local\",\"time\":1,\"timeNano\":1000000000}\n"
        }
        "/v1.49/containers/container-fixture/stats?stream=true" => {
            "{\"id\":\"container-fixture\",\"name\":\"fixture\",\"cpu_stats\":{\"cpu_usage\":{\"total_usage\":2},\"system_cpu_usage\":4,\"online_cpus\":2},\"precpu_stats\":{\"cpu_usage\":{\"total_usage\":1},\"system_cpu_usage\":2,\"online_cpus\":2},\"memory_stats\":{\"usage\":4096,\"limit\":8192},\"networks\":{}}\n"
        }
        "/v1.49/containers/container-fixture/stats?stream=false&one-shot=true" => {
            r#"{"id":"container-fixture","memory_stats":{"usage":4096,"limit":8192}}"#
        }
        unexpected => panic!("unexpected request {unexpected}"),
    };
    body.as_bytes().to_vec()
}

fn temp_socket() -> PathBuf {
    std::env::temp_dir().join(format!("citadel-phase0-{}.sock", Uuid::now_v7()))
}
