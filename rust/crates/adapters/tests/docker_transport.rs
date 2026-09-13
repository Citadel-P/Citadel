#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use citadel_platforms::{CreateRuntimeNetwork, CreateRuntimeVolume, PlatformResourceMutationPort};
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use uuid::Uuid;

#[path = "docker_transport/container_mutations.rs"]
mod container_mutations;
#[path = "docker_transport/distribution.rs"]
mod distribution;

#[tokio::test]
async fn volume_sizes_are_joined_by_name_from_volume_only_disk_usage() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for (expected, body) in [
            (
                "GET /version ",
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            ),
            (
                "GET /v1.49/volumes ",
                r#"{"Volumes":[{"Name":"data"},{"Name":"empty"},{"Name":"unknown"}]}"#,
            ),
            (
                "GET /v1.49/system/df?type=volume ",
                r#"{"Volumes":[{"Name":"empty","UsageData":{"Size":0,"RefCount":0}},{"Name":"data","UsageData":{"Size":4096,"RefCount":2}}]}"#,
            ),
            ("GET /v1.49/volumes/data ", r#"{"Name":"data"}"#),
            (
                "GET /v1.49/system/df?type=volume ",
                r#"{"Volumes":[{"Name":"data","UsageData":{"Size":8192,"RefCount":2}}]}"#,
            ),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 2048];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 8192);
                bytes.extend_from_slice(&buffer[..n]);
            }
            assert!(String::from_utf8_lossy(&bytes).starts_with(expected));
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
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let volumes = citadel_platforms::PlatformInventoryPort::list_volumes(&client, &cancel)
        .await
        .unwrap();
    assert_eq!(volumes[0].usage_data.as_ref().unwrap()["Size"], 4096);
    assert!(volumes[0].in_use);
    assert_eq!(volumes[1].usage_data.as_ref().unwrap()["Size"], 0);
    assert!(!volumes[1].in_use);
    assert!(volumes[2].usage_data.is_none());
    let inspected =
        citadel_platforms::PlatformInventoryPort::inspect_volume(&client, "data", &cancel)
            .await
            .unwrap();
    assert_eq!(inspected.usage_data.unwrap()["Size"], 8192);
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn network_create_preserves_ip_versions_and_omits_empty_ipam_rows() {
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for step in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let header_end = loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let body = if step == 0 {
                assert!(headers.starts_with("GET /version "));
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else {
                assert!(headers.starts_with("POST /v1.49/networks/create "));
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                while bytes.len() < header_end + length {
                    let n = socket.read(&mut buffer).await.unwrap();
                    assert!(n > 0 && bytes.len() < 16384);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let request: serde_json::Value =
                    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                assert_eq!(request["EnableIPv4"], true);
                assert_eq!(request["EnableIPv6"], false);
                assert_eq!(
                    request["IPAM"]["Config"],
                    serde_json::json!([{"Subnet":"10.42.0.0/24"}])
                );
                r#"{"Id":"created","Warning":""}"#
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
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let input = serde_json::from_value(serde_json::json!({
        "name":"created", "driver":"bridge", "scope":"local",
        "enableIPv4":true, "enableIPv6":false,
        "ipam":{"driver":"default", "config":[{}, {}, {"subnet":"10.42.0.0/24"}]}
    }))
    .unwrap();
    let created = PlatformResourceMutationPort::create_network(
        &client,
        &input,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(created.id, "created");
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn image_inspect_combines_generated_details_history_and_filtered_containers() {
    use citadel_platforms::images::ImageInspectionPort;
    use tokio_util::sync::CancellationToken;
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let body = if request.starts_with("GET /version ") {
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else if request.starts_with("GET /v1.49/images/sha256%3Aabc/json ") {
                r#"{"Id":"sha256:abc","RepoTags":["registry:5000/app:v1"],"Config":{"Env":null,"Cmd":null,"Volumes":{"/data":{}},"ExposedPorts":{"80/tcp":{}},"Labels":null}}"#
            } else if request.starts_with("GET /v1.49/images/sha256%3Aabc/history ") {
                r#"[{"Id":"layer","Created":123,"CreatedBy":"COPY","Size":42,"Comment":"base"}]"#
            } else {
                assert!(
                    request.starts_with("GET /v1.49/containers/json?all=true&filters="),
                    "{request}"
                );
                assert!(
                    urlencoding::decode(&request)
                        .unwrap()
                        .contains(r#""ancestor":["sha256:abc"]"#)
                );
                r#"[{"Id":"container","Names":["/web"],"State":"running","Mounts":[{"Name":"data"},{"Source":"/bind"}],"NetworkSettings":{"Networks":{"bridge":{"NetworkID":"network-id"}}},"Ports":[{"PrivatePort":80,"PublicPort":8080,"Type":"tcp","IP":"::"}]}]"#
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
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let image =
        ImageInspectionPort::inspect_image(&client, "sha256:abc", &CancellationToken::new())
            .await
            .unwrap();
    assert_eq!(image.name, "registry:5000/app");
    assert_eq!(image.tag, "v1");
    assert!(image.env.is_empty() && image.labels.is_empty());
    assert_eq!(image.layers[0].size, 42);
    assert_eq!(image.containers[0].volumes, vec!["data"]);
    assert_eq!(image.containers[0].networks["bridge"], "network-id");
    assert_eq!(image.containers[0].ports["80/tcp"][0]["hostIP"], "::");
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn container_logs_negotiate_inspect_tty_and_decode_the_generated_stream_route() {
    use citadel_platforms::logs::ContainerLogPort;
    use tokio_util::sync::CancellationToken;
    for tty in [false, true] {
        let path = temp_socket();
        let listener = UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            for step in 0..3 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = vec![];
                let mut buffer = [0; 4096];
                while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = socket.read(&mut buffer).await.unwrap();
                    assert!(n > 0 && bytes.len() < 16384);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8(bytes).unwrap();
                let body = match step {
                    0 => {
                        assert!(request.starts_with("GET /version "));
                        br#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#.to_vec()
                    }
                    1 => {
                        assert!(request.starts_with("GET /v1.49/containers/container-1/json "));
                        format!(r#"{{"Config":{{"Tty":{tty}}}}}"#).into_bytes()
                    }
                    _ => {
                        assert!(request.starts_with("GET /v1.49/containers/container-1/logs?stdout=true&stderr=true&timestamps=true&follow=true&tail=100&details=false "));
                        let log = b"2026-09-06T12:00:00Z hello\n";
                        if tty {
                            log.to_vec()
                        } else {
                            let mut b = vec![1, 0, 0, 0];
                            b.extend_from_slice(&(log.len() as u32).to_be_bytes());
                            b.extend_from_slice(log);
                            b
                        }
                    }
                };
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                socket.write_all(&body).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        });
        let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
        let mut logs = client
            .container_logs("container-1", &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            logs.next().await.unwrap().unwrap(),
            b"2026-09-06T12:00:00Z hello\n"
        );
        assert!(logs.next().await.is_none());
        server.await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn cancelled_task_inspection_does_not_open_the_docker_socket() {
    use citadel_platforms::terminal::{ContainerTerminalPort, TerminalShell};
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
    assert!(
        matches!(client.container_terminal("container", TerminalShell::Sh, &cancellation).await,
        Err(error) if error.kind == RuntimeErrorKind::Cancelled)
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
        for _ in 0..30 {
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
            "/v1.49/system/df?type=volume",
            "/v1.49/volumes/data",
            "/v1.49/system/df?type=volume",
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
        "/v1.49/system/df?type=volume" => {
            r#"{"Volumes":[{"Name":"data","UsageData":{"Size":1024,"RefCount":1}}]}"#
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

#[tokio::test]
async fn terminal_hijacks_generated_exec_route_streams_stdin_resizes_and_closes_on_drop() {
    exercise_terminal_resize("200 OK").await;
}

// .NET ContainerService.ExecAsync logs a resize failure without disposing the
// interactive stream. Docker can reject an initial resize during exec startup.
#[tokio::test]
async fn terminal_resize_rejection_does_not_close_the_interactive_session() {
    exercise_terminal_resize("409 Conflict").await;
}

async fn exercise_terminal_resize(resize_status: &'static str) {
    use citadel_platforms::terminal::*;
    use tokio_util::sync::CancellationToken;
    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        for step in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (request, body) = read_terminal_request(&mut socket).await;
            let response = if step == 0 {
                assert!(request.starts_with("GET /version "));
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else {
                assert!(request.starts_with("POST /v1.49/containers/container-1/exec "));
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(body["Cmd"], serde_json::json!(["/bin/sh"]));
                assert_eq!(body["Tty"], true);
                assert_eq!(body["AttachStdin"], true);
                r#"{"Id":"exec-1"}"#
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).as_bytes()).await.unwrap();
        }
        let (mut terminal, _) = listener.accept().await.unwrap();
        let (request, body) = read_terminal_request(&mut terminal).await;
        assert!(request.starts_with("POST /v1.49/exec/exec-1/start "));
        assert!(request.to_ascii_lowercase().contains("upgrade: tcp"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"Detach":false,"Tty":true})
        );
        terminal
            .write_all(b"HTTP/1.1 101 UPGRADED\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n")
            .await
            .unwrap();
        let mut input = [0; 3];
        terminal.read_exact(&mut input).await.unwrap();
        assert_eq!(&input, b"ls\n");
        terminal.write_all(b"hello\n").await.unwrap();
        let (mut resize, _) = listener.accept().await.unwrap();
        let (request, _) = read_terminal_request(&mut resize).await;
        assert!(request.starts_with("POST /v1.49/exec/exec-1/resize?w=100&h=30 "));
        resize
            .write_all(
                format!(
                    "HTTP/1.1 {resize_status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        terminal.write_all(b"resized").await.unwrap();
        assert_eq!(terminal.read(&mut input).await.unwrap(), 0);
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let mut terminal = client
        .container_terminal("container-1", TerminalShell::Sh, &CancellationToken::new())
        .await
        .unwrap();
    terminal
        .input
        .try_send(TerminalInput::Stdin(b"ls\n".to_vec()))
        .unwrap();
    assert!(
        matches!(terminal.output.next().await.unwrap().unwrap(),TerminalOutput::Data(bytes) if bytes==b"hello\n")
    );
    terminal
        .input
        .try_send(TerminalInput::Resize {
            cols: 100,
            rows: 30,
        })
        .unwrap();
    assert!(
        matches!(terminal.output.next().await.unwrap().unwrap(),TerminalOutput::Data(bytes) if bytes==b"resized")
    );
    drop(terminal.output);
    assert!(terminal.input.is_closed());
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    std::fs::remove_file(path).unwrap();
}

async fn read_terminal_request(socket: &mut tokio::net::UnixStream) -> (String, Vec<u8>) {
    // Read headers and the complete JSON body before switching this fixture to raw I/O.
    let mut bytes = Vec::new();
    let mut one = [0; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        socket.read_exact(&mut one).await.unwrap();
        bytes.push(one[0]);
        assert!(bytes.len() < 16384);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .and_then(|value| value.parse::<usize>().ok())
        })
        .unwrap_or(0);
    assert!(length < 16384);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.unwrap();
    (headers, body)
}

#[tokio::test]
async fn platform_counts_follow_visible_lists_in_info_and_live_stats() {
    use citadel_platforms::PlatformRuntimePort;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio_util::sync::CancellationToken;

    let path = temp_socket();
    let listener = UnixListener::bind(&path).unwrap();
    let phase = Arc::new(AtomicUsize::new(0));
    let server_phase = phase.clone();
    let stop = CancellationToken::new();
    let server_stop = stop.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = tokio::select! {
                () = server_stop.cancelled() => break,
                socket = listener.accept() => socket.unwrap(),
            };
            let mut bytes = Vec::new();
            let mut buffer = [0; 2048];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 8192);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let route = request.split_whitespace().nth(1).unwrap();
            let mut status = "200 OK";
            let body = match route {
                "/version" => r#"{"Version":"fixture","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#.to_owned(),
                "/v1.49/info" => r#"{"ID":"desktop","Images":33,"Containers":57,"ContainersRunning":47,"ContainersPaused":0,"ContainersStopped":10}"#.to_owned(),
                "/v1.49/containers/json?all=true" => match server_phase.load(Ordering::Acquire) {
                    0 => serde_json::to_string(&[
                        "running", "running", "running", "running", "running", "running", "running", "running",
                        "exited", "exited", "exited", "exited", "exited", "exited", "exited", "exited", "exited",
                        "paused", "created", "restarting",
                    ].iter().map(|state| serde_json::json!({"State":state})).collect::<Vec<_>>()).unwrap(),
                    2 => { status = "500 Internal Server Error"; r#"{"message":"inventory unavailable"}"#.to_owned() },
                    _ => "[]".to_owned(),
                },
                "/v1.49/images/json?all=true" => match server_phase.load(Ordering::Acquire) {
                    0 => serde_json::to_string(&(0..32).map(|id| serde_json::json!({
                        "Id": format!("image-{id}"),
                        "RepoTags": if id == 0 { vec!["app:latest", "app:stable"] } else { vec![] },
                    })).collect::<Vec<_>>()).unwrap(),
                    3 => { status = "500 Internal Server Error"; r#"{"message":"images unavailable"}"#.to_owned() },
                    _ => "[]".to_owned(),
                },
                "/v1.49/networks" => if server_phase.load(Ordering::Acquire) == 0 { r#"[{"Id":"bridge"},{"Id":"app"}]"# } else { "[]" }.to_owned(),
                "/v1.49/volumes" => if server_phase.load(Ordering::Acquire) == 0 { r#"{"Volumes":[{"Name":"data"},{"Name":"anonymous"},{"Name":"unused"}]}"# } else { r#"{"Volumes":[]}"# }.to_owned(),
                "/v1.49/system/df?type=image&type=volume" => r#"{"LayersSize":0,"Volumes":[]}"#.to_owned(),
                other => panic!("unexpected request {other}"),
            };
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
    let info = PlatformRuntimePort::get_info(&client, &stop).await.unwrap();
    assert_eq!(
        (
            info.container_count,
            info.containers_running,
            info.containers_paused,
            info.containers_stopped
        ),
        (20, 8, 1, 9)
    );
    let stats = client.platform_stats().await.unwrap();
    assert_eq!((stats.network_count, stats.volume_count), (2, 3));
    assert_eq!(
        stats.image_count, 32,
        "count image identities, including untagged images, not /info totals or tags"
    );
    assert_eq!(
        (
            stats.container_count,
            stats.containers_running,
            stats.containers_paused,
            stats.containers_stopped
        ),
        (20, 8, 1, 9)
    );
    phase.store(1, Ordering::Release);
    let mut stream = PlatformRuntimePort::stream_stats(&client, Duration::from_millis(10), &stop)
        .await
        .unwrap();
    let stats = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            stats.container_count,
            stats.containers_running,
            stats.containers_paused,
            stats.containers_stopped
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(
        stats.image_count, 0,
        "an empty image list must clear the previous count"
    );
    assert_eq!((stats.network_count, stats.volume_count), (0, 0));
    drop(stream);
    phase.store(2, Ordering::Release);
    assert!(
        client.platform_stats().await.is_err(),
        "failed listing must not publish fabricated counts or fall back to /info"
    );
    phase.store(3, Ordering::Release);
    assert!(
        client.platform_stats().await.is_err(),
        "failed image listing must not fall back to /info or publish zero"
    );
    stop.cancel();
    server.await.unwrap();
    std::fs::remove_file(path).unwrap();
}
