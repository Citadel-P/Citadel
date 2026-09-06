#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use citadel_platforms::{CreateRuntimeNetwork, CreateRuntimeVolume, PlatformResourceMutationPort};
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use uuid::Uuid;

#[tokio::test]
async fn cancelled_task_inspection_does_not_open_the_docker_socket() {
    use citadel_platforms::{RuntimeErrorKind, SwarmTaskRuntimePort};
    let socket = temp_socket();
    let client = DockerClient::new(&socket, Duration::from_secs(1)).unwrap();
    let cancellation = tokio_util::sync::CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        client
            .inspect_task("task", &cancellation)
            .await
            .unwrap_err()
            .kind,
        RuntimeErrorKind::Cancelled
    );
}

#[tokio::test]
async fn node_agent_setup_uses_generated_distribution_and_create_routes() {
    use serde_json::json;
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let mut calls = vec![];
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            let header_end = loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
            let line = header.lines().next().unwrap().to_owned();
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(|n| n.parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let body = if line.starts_with("GET /version ") {
                json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})
            } else if line.starts_with("GET ") {
                json!({"Descriptor":{"digest":"sha256:abc"}})
            } else {
                let sent: serde_json::Value =
                    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                assert_eq!(sent["Data"], "dGVzdA==");
                assert_eq!(sent["Labels"]["com.citadel.system"], "true");
                json!({"ID":"created"})
            }
            .to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
            calls.push(line);
        }
        calls
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    client
        .distribution_inspect("ghcr.io/citadel-p/agent:latest")
        .await
        .unwrap();
    let spec = json!({"Name":"bootstrap","Data":"dGVzdA==","Labels":{"com.citadel.system":"true"}});
    assert_eq!(
        client.create_swarm_material(true, &spec).await.unwrap()["ID"],
        "created"
    );
    assert_eq!(
        client.create_swarm_material(false, &spec).await.unwrap()["ID"],
        "created"
    );
    let calls = server.await.unwrap();
    assert!(
        calls[1].starts_with("GET /v1.49/distribution/ghcr.io%2Fcitadel-p%2Fagent%3Alatest/json "),
        "{:?}",
        calls
    );
    assert!(calls[2].starts_with("POST /v1.49/secrets/create "));
    assert!(calls[3].starts_with("POST /v1.49/configs/create "));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn node_agent_resources_use_generated_inspect_and_delete_contracts() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let mut calls = Vec::new();
        for _ in 0..7 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0 && request.len() < 16 * 1024);
                request.extend_from_slice(&chunk[..n]);
            }
            let line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap()
                .to_owned();
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap();
            let resource = parts.next().unwrap();
            let (status, body) = if resource == "/version" {
                ("200 OK", r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#)
            } else if method == "DELETE" {
                ("204 No Content", "")
            } else {
                (
                    "200 OK",
                    r#"{"ID":"owned","Version":{"Index":1},"Spec":{"Labels":{"com.citadel.system":"true"}}}"#,
                )
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
            calls.push(format!("{method} {resource}"));
        }
        calls
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    assert_eq!(
        client.inspect_swarm_service("owned").await.unwrap().spec["Labels"]["com.citadel.system"],
        "true"
    );
    client.delete_swarm_service("owned").await.unwrap();
    assert_eq!(
        client.inspect_swarm_secret("owned").await.unwrap().spec["Labels"]["com.citadel.system"],
        "true"
    );
    client.delete_swarm_secret("owned").await.unwrap();
    assert_eq!(
        client.inspect_swarm_config("owned").await.unwrap().spec["Labels"]["com.citadel.system"],
        "true"
    );
    client.delete_swarm_config("owned").await.unwrap();
    assert_eq!(
        server.await.unwrap(),
        [
            "GET /version",
            "GET /v1.49/services/owned",
            "DELETE /v1.49/services/owned",
            "GET /v1.49/secrets/owned",
            "DELETE /v1.49/secrets/owned",
            "GET /v1.49/configs/owned",
            "DELETE /v1.49/configs/owned"
        ]
    );
    assert!(client.inspect_swarm_secret("").await.is_err());
    assert!(client.delete_swarm_config("").await.is_err());
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn generated_subset_uses_versioned_unix_socket_requests_and_bounded_streams() {
    let socket_path = temp_socket();
    let listener = UnixListener::bind(&socket_path).unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for _ in 0..28 {
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
            if path.contains("/images/create") {
                let headers = String::from_utf8_lossy(&request);
                assert!(
                    headers
                        .to_ascii_lowercase()
                        .contains("x-registry-auth: registry-auth")
                );
            }
            if path.ends_with("/networks/create") || path.ends_with("/volumes/create") {
                assert!(first_line.starts_with("POST "));
            }
            if path.contains("/containers/create")
                || path.contains("/start")
                || path.contains("/images/create")
            {
                assert!(first_line.starts_with("POST "));
            }
            if path.contains("/networks/network-created")
                || path.contains("/volumes/created")
                || path.contains("/containers/container-delete")
            {
                assert!(first_line.starts_with("DELETE "));
            }
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
    client
        .delete_container("container-delete", true, true)
        .await
        .unwrap();
    let created = client
        .create_container("web", &serde_json::json!({"Image":"sha256:image"}))
        .await
        .unwrap();
    assert_eq!(created, "container-created");
    client.start_container(&created).await.unwrap();
    let mut pull = client
        .pull_image("nginx:latest", Some("registry-auth"))
        .await
        .unwrap();
    assert_eq!(
        pull.next().await.unwrap().unwrap().status.as_deref(),
        Some("Downloaded")
    );
    assert_eq!(client.list_images().await.unwrap()[0].id, "sha256:image");
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
    let cancellation = tokio_util::sync::CancellationToken::new();
    assert_eq!(
        PlatformResourceMutationPort::create_network(
            &client,
            &CreateRuntimeNetwork {
                name: "created".into(),
                driver: "bridge".into(),
                scope: "local".into(),
                internal: None,
                attachable: None,
                ingress: None,
                enable_ipv6: None,
                enable_ipv4: Some(true),
                config_only: None,
                ipam: None,
                config_from: None,
                labels: Default::default(),
                options: Default::default(),
            },
            &cancellation,
        )
        .await
        .unwrap()
        .id,
        "network-created"
    );
    PlatformResourceMutationPort::delete_network(&client, "network-created", &cancellation)
        .await
        .unwrap();
    assert_eq!(
        PlatformResourceMutationPort::create_volume(
            &client,
            &CreateRuntimeVolume {
                name: "created".into(),
                driver: "local".into(),
                labels: Default::default(),
                options: Default::default(),
            },
            &cancellation,
        )
        .await
        .unwrap()
        .name,
        "created"
    );
    PlatformResourceMutationPort::delete_volume(&client, "created", false, &cancellation)
        .await
        .unwrap();
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
            "/v1.49/containers/container-delete?v=true&force=true&link=false",
            "/v1.49/containers/create?name=web",
            "/v1.49/containers/container-created/start",
            "/v1.49/images/create?fromImage=nginx%3Alatest",
            "/v1.49/images/json?all=true",
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
            "/v1.49/networks/create",
            "/v1.49/networks/network-created",
            "/v1.49/volumes/create",
            "/v1.49/volumes/created?force=false",
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
            r#"{"ID":"daemon-fixture","Containers":1,"ContainersRunning":1,"ContainersStopped":0,"ContainersPaused":0,"Images":1,"NCPU":2,"MemTotal":1073741824,"Swarm":{"LocalNodeState":"inactive","RemoteManagers":null}}"#
        }
        "/v1.49/containers/json?all=true" => {
            r#"[{"Id":"container-fixture","Names":["/fixture"],"Image":"nginx:alpine","ImageID":"sha256:fixture","Created":1,"Labels":{},"State":"running","Status":"Up"}]"#
        }
        "/v1.49/containers/container-fixture/json" => {
            r#"{"Id":"container-fixture","Created":"2026-01-01T00:00:00Z","Path":"nginx","Args":[],"State":{"Status":"running","Running":true},"Image":"sha256:fixture","Name":"/fixture","Config":{"Image":"nginx:alpine","Labels":{}}}"#
        }
        "/v1.49/containers/container-delete?v=true&force=true&link=false" => "",
        "/v1.49/containers/create?name=web" => r#"{"Id":"container-created","Warnings":[]}"#,
        "/v1.49/containers/container-created/start" => "",
        "/v1.49/images/create?fromImage=nginx%3Alatest" => {
            "{\"status\":\"Downloaded\",\"id\":\"sha256:image\"}\n"
        }
        "/v1.49/swarm" => {
            r#"{"ID":"swarm-fixture","Version":{"Index":7},"CreatedAt":"2026-01-01T00:00:00Z","UpdatedAt":"2026-01-02T00:00:00Z","JoinTokens":{"Worker":"worker-secret","Manager":"manager-secret"}}"#
        }
        "/v1.49/images/json?all=true" => {
            r#"[{"Id":"sha256:image","RepoTags":["nginx:alpine"],"RepoDigests":[],"Created":1,"Size":10,"Labels":{},"Containers":1}]"#
        }
        "/v1.49/volumes" => {
            r#"{"Volumes":[{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Status":null,"Labels":null,"Scope":"local","Options":null}],"Warnings":null}"#
        }
        "/v1.49/volumes/data" => {
            r#"{"Name":"data","Driver":"local","Mountpoint":"/data","CreatedAt":"2026-01-01T00:00:00Z","Status":null,"Labels":null,"Scope":"local","Options":null}"#
        }
        "/v1.49/networks" => {
            r#"[{"Name":"bridge","Id":"network-1","Created":"2026-01-01T00:00:00Z","Scope":"local","Driver":"bridge","EnableIPv4":true,"Containers":null,"Peers":null,"Labels":{},"Options":{}}]"#
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
        "/v1.49/networks/create" => r#"{"Id":"network-created","Warning":""}"#,
        "/v1.49/networks/network-created" => "",
        "/v1.49/volumes/create" => {
            r#"{"Name":"created","Driver":"local","Mountpoint":"/var/lib/docker/volumes/created/_data","CreatedAt":"2026-01-01T00:00:00Z","Labels":{},"Scope":"local","Options":{}}"#
        }
        "/v1.49/volumes/created?force=false" => "",
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
