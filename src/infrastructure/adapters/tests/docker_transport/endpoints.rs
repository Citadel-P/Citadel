use super::*;
use citadel_adapters::connectors::docker::{DockerEndpoint, DockerError};
use tokio::net::{TcpListener, TcpStream};

async fn request(socket: &mut TcpStream) -> String {
    request_details(socket).await.0
}
async fn request_details(socket: &mut TcpStream) -> (String, Vec<u8>) {
    let mut bytes = Vec::new();
    loop {
        bytes.push(socket.read_u8().await.unwrap());
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        assert!(bytes.len() < 16384);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.unwrap();
    (headers, body)
}
async fn respond(socket: &mut TcpStream, body: &str) {
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

#[tokio::test]
async fn deployment_apply_observes_state_after_an_ambiguous_start_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for step in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, body) = request_details(&mut socket).await;
            match step {
                0 => {
                    respond(
                        &mut socket,
                        r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                    )
                    .await
                }
                1 => {
                    assert!(headers.contains("/containers/create?name=test"));
                    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(body["Image"], "test:v1");
                    assert_eq!(body["HostConfig"]["Memory"], 104857600);
                    respond(&mut socket, r#"{"Id":"created"}"#).await;
                }
                2 => {
                    assert!(headers.contains("/containers/created/start"));
                    socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
                }
                3 => {
                    assert!(headers.contains("/containers/created/json"));
                    respond(
                        &mut socket,
                        r#"{"Id":"created","State":{"Running":true,"Status":"running"}}"#,
                    )
                    .await;
                }
                _ => unreachable!(),
            }
        }
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let result = docker
        .apply_container_config(
            "test",
            "image-id",
            &serde_json::json!({"Image":"test:v1","HostConfig":{"Memory":104857600}}),
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(result.docker_container_id, "created");
    assert_eq!(result.docker_image_id, "image-id");
    assert_eq!(
        result.state,
        citadel_deployments::RuntimeContainerState::Running
    );
    cancel.cancel();
    assert!(matches!(
        docker
            .apply_container_config("test", "image-id", &serde_json::json!({}), &cancel)
            .await,
        Err(citadel_deployments::DeploymentError::Cancelled)
    ));
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn filtered_lists_preserve_volume_warnings_and_include_nonrunning_tasks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for step in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let headers = request(&mut socket).await;
            let uri = url::Url::parse(&format!(
                "http://fixture{}",
                headers.split_whitespace().nth(1).unwrap()
            ))
            .unwrap();
            let query = uri
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            let body = match step {
                0 => r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                1 => {
                    assert_eq!(uri.path(), "/v1.49/volumes");
                    assert_eq!(query["filters"], r#"{"label":["app=test"]}"#);
                    r#"{"Volumes":[],"Warnings":["driver unavailable"]}"#
                }
                2 => {
                    assert_eq!(uri.path(), "/v1.49/tasks");
                    assert!(!query.contains_key("filters"));
                    r#"[{"ID":"old-task","DesiredState":"shutdown"}]"#
                }
                3 => {
                    assert_eq!(uri.path(), "/v1.49/networks");
                    assert_eq!(
                        query["filters"],
                        r#"{"scope":["swarm"],"label":["app=test"]}"#
                    );
                    "[]"
                }
                _ => unreachable!(),
            };
            respond(&mut socket, body).await;
        }
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    let volumes = docker
        .list_volume_models(Some(r#"{"label":["app=test"]}"#))
        .await
        .unwrap();
    assert_eq!(volumes.warnings.unwrap(), ["driver unavailable"]);
    assert_eq!(
        docker.list_swarm_tasks_filtered(None).await.unwrap().len(),
        1
    );
    assert!(
        docker
            .list_networks_filtered(Some(r#"{"scope":["swarm"],"label":["app=test"]}"#))
            .await
            .unwrap()
            .is_empty()
    );
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn pull_import_options_and_task_log_snapshots_use_the_selected_daemon() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for step in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let headers = request(&mut socket).await;
            let uri = url::Url::parse(&format!(
                "http://fixture{}",
                headers.split_whitespace().nth(1).unwrap()
            ))
            .unwrap();
            let query = uri
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            let body = match step {
                0 => {
                    r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41","Components":[{"Name":"BuildKit","Version":"test"}]}"#
                }
                1 => {
                    assert_eq!(uri.path(), "/v1.49/images/create");
                    assert_eq!(query["fromSrc"], "https://example.test/rootfs.tar");
                    assert_eq!(query["repo"], "test");
                    assert_eq!(query["tag"], "v1");
                    assert_eq!(query["changes"], "ENV APP=test");
                    "{\"errorDetail\":{\"code\":42,\"message\":\"import failed\"}}\n"
                }
                2 => {
                    assert_eq!(uri.path(), "/v1.49/tasks/task-id");
                    r#"{"Spec":{"ContainerSpec":{"TTY":true}}}"#
                }
                3 => {
                    assert_eq!(uri.path(), "/v1.49/tasks/task-id/logs");
                    assert_eq!(query["follow"], "false");
                    assert_eq!(query["tail"], "25");
                    "task output\n"
                }
                _ => unreachable!(),
            };
            respond(&mut socket, body).await;
        }
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    assert_eq!(
        docker.version().await.unwrap().components[0].name,
        "BuildKit"
    );
    let mut pull = docker
        .pull_image_with_options(
            citadel_adapters::connectors::docker::DockerImagePullOptions {
                from_source: Some("https://example.test/rootfs.tar"),
                repository: Some("test"),
                tag: Some("v1"),
                changes: &["ENV APP=test".into()],
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        pull.next()
            .await
            .unwrap()
            .unwrap()
            .error_detail
            .unwrap()
            .code,
        Some(42)
    );
    assert!(pull.next().await.is_none());
    let mut logs = docker
        .task_logs("task-id", 25, &tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(logs.next().await.unwrap().unwrap(), b"task output\n");
    assert!(logs.next().await.is_none());
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn tcp_uses_the_selected_daemon_for_generated_calls_inspection_and_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint: DockerEndpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for step in 0..6 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let headers = request(&mut socket).await;
            let uri = url::Url::parse(&format!(
                "http://fixture{}",
                headers.split_whitespace().nth(1).unwrap()
            ))
            .unwrap();
            let query = uri
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            let body = match step {
                0 => {
                    assert_eq!(uri.path(), "/_ping");
                    "OK"
                }
                1 => {
                    assert_eq!(uri.path(), "/version");
                    r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
                }
                2 => {
                    assert_eq!(uri.path(), "/v1.49/containers/json");
                    assert_eq!(query["all"], "false");
                    assert_eq!(query["limit"], "3");
                    assert_eq!(query["size"], "true");
                    assert_eq!(query["filters"], r#"{"label":["app=test"]}"#);
                    "[]"
                }
                3 => {
                    assert_eq!(uri.path(), "/v1.49/containers/container-1/json");
                    r#"{"Id":"container-1"}"#
                }
                4 => {
                    assert_eq!(uri.path(), "/v1.49/events");
                    assert_eq!(query["since"], "123");
                    "{\"Type\":\"container\",\"Action\":\"start\"}\n"
                }
                5 => {
                    assert_eq!(uri.path(), "/v1.49/images/create");
                    assert_eq!(query["fromImage"], "test:latest");
                    assert!(
                        headers
                            .to_ascii_lowercase()
                            .contains("x-registry-auth: fixture")
                    );
                    "{\"status\":\"pulled\"}\n"
                }
                _ => unreachable!(),
            };
            respond(&mut socket, body).await;
        }
    });
    let client = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    client.ping().await.unwrap();
    assert!(
        client
            .list_container_models(
                Some(false),
                Some(3),
                Some(true),
                Some(r#"{"label":["app=test"]}"#)
            )
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client
            .inspect_container_document("container-1")
            .await
            .unwrap()["Id"],
        "container-1"
    );
    let mut events = client.events(Some(123), None).await.unwrap();
    assert!(events.next().await.unwrap().is_ok());
    assert!(events.next().await.is_none());
    let mut pull = client
        .pull_image("test:latest", Some("fixture"))
        .await
        .unwrap();
    assert_eq!(
        pull.next().await.unwrap().unwrap().status.as_deref(),
        Some("pulled")
    );
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn tcp_upgraded_terminal_uses_the_same_endpoint_and_resize_connection() {
    use citadel_platforms::terminal::{
        ContainerTerminalPort, TerminalInput, TerminalOutput, TerminalShell,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for (route, body) in [
            (
                "/version",
                r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            ),
            ("/v1.49/containers/test/exec", r#"{"Id":"exec-1"}"#),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(request(&mut socket).await.contains(route));
            respond(&mut socket, body).await;
        }
        let (mut stream, _) = listener.accept().await.unwrap();
        assert!(
            request(&mut stream)
                .await
                .contains("/v1.49/exec/exec-1/start")
        );
        stream
            .write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n",
            )
            .await
            .unwrap();
        let mut input = [0; 3];
        stream.read_exact(&mut input).await.unwrap();
        assert_eq!(&input, b"ls\n");
        stream.write_all(b"hello").await.unwrap();
        let (mut resize, _) = listener.accept().await.unwrap();
        assert!(
            request(&mut resize)
                .await
                .contains("/v1.49/exec/exec-1/resize?h=30&w=100")
        );
        respond(&mut resize, "").await;
        stream.write_all(b"resized").await.unwrap();
        assert_eq!(stream.read(&mut input).await.unwrap(), 0);
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut terminal = docker
        .container_terminal("test", TerminalShell::Sh, &cancel)
        .await
        .unwrap();
    terminal
        .input
        .try_send(TerminalInput::Stdin(b"ls\n".to_vec()))
        .unwrap();
    assert!(
        matches!(terminal.output.next().await.unwrap().unwrap(),TerminalOutput::Data(bytes) if bytes==b"hello")
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
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn daemon_redirects_are_not_followed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let redirect = endpoint.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {redirect}/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    });
    let client =
        DockerClient::with_endpoint(endpoint.parse().unwrap(), Duration::from_secs(2), "/host")
            .unwrap();
    assert!(
        matches!(client.ping().await,Err(DockerError::Api { status, .. }) if status.as_u16()==302)
    );
    server.await.unwrap();
}

#[test]
fn endpoint_validation_and_cli_addresses_do_not_fall_back_to_localhost() {
    for value in [
        "tcp://host",
        "https://host:2376",
        "tcp://user:secret@host:2375",
        "unix:///",
        "unix:///tmp/%00socket",
        "http://host/path",
        "http://host?query=1",
    ] {
        assert!(value.parse::<DockerEndpoint>().is_err(), "{value}");
    }
    for (value, expected) in [
        ("http://docker", "tcp://docker:80"),
        ("tcp://[::1]:2375", "tcp://[::1]:2375"),
        ("unix:///tmp/test%20socket", "unix:///tmp/test%20socket"),
    ] {
        assert_eq!(
            value
                .parse::<DockerEndpoint>()
                .unwrap()
                .docker_host()
                .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn binary_exec_preserves_options_bytes_stream_identity_and_exit_code() {
    use citadel_adapters::connectors::docker::DockerExecEvent;
    for tty in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let server = tokio::spawn(async move {
            for step in 0..4 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let (headers, body) = request_details(&mut socket).await;
                match step {
                    0 => {
                        respond(
                            &mut socket,
                            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                        )
                        .await
                    }
                    1 => {
                        assert!(headers.contains("/containers/test/exec"));
                        let config: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(config["Cmd"], serde_json::json!(["cat", "/data/archive"]));
                        assert_eq!(config["Env"], serde_json::json!(["MODE=binary"]));
                        assert_eq!(config["AttachStdin"], false);
                        assert_eq!(config["AttachStdout"], false);
                        assert_eq!(config["AttachStderr"], true);
                        assert_eq!(config["Tty"], tty);
                        respond(&mut socket, r#"{"Id":"exec-1"}"#).await;
                    }
                    2 => {
                        assert!(headers.contains("/exec/exec-1/start"));
                        assert_eq!(
                            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["Tty"],
                            tty
                        );
                        socket.write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n").await.unwrap();
                        if !tty {
                            for byte in [2, 0, 0, 0, 0, 0, 0, 3] {
                                socket.write_u8(byte).await.unwrap();
                            }
                        }
                        socket.write_all(&[0, 255, 42]).await.unwrap();
                    }
                    3 => {
                        assert!(headers.contains("/exec/exec-1/json"));
                        respond(&mut socket, r#"{"Running":false,"ExitCode":7}"#).await;
                    }
                    _ => unreachable!(),
                }
            }
        });
        let docker =
            DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
        let mut output = docker
            .execute_binary(
                "test",
                citadel_docker_api::models::ExecConfig {
                    cmd: Some(vec!["cat".into(), "/data/archive".into()]),
                    env: Some(vec!["MODE=binary".into()]),
                    attach_stdout: Some(false),
                    attach_stderr: Some(true),
                    tty: Some(tty),
                    ..Default::default()
                },
                &tokio_util::sync::CancellationToken::new(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        let mut exits = 0;
        while let Some(event) = output.next().await {
            match event.unwrap() {
                DockerExecEvent::Output { data, stderr } => {
                    bytes.extend(data);
                    assert_eq!(stderr, !tty);
                }
                DockerExecEvent::Exit(code) => {
                    assert_eq!(code, 7);
                    exits += 1;
                }
            }
        }
        assert_eq!(bytes, [0, 255, 42]);
        assert_eq!(exits, 1);
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn cancelling_exec_interrupts_a_partial_multiplex_header() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for body in [
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            r#"{"Id":"exec-1"}"#,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            respond(&mut socket, body).await;
        }
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        socket.write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n\x01").await.unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).await.unwrap(), 0);
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut stream = docker
        .execute_binary(
            "test",
            citadel_docker_api::models::ExecConfig {
                cmd: Some(vec!["cat".into()]),
                attach_stdout: Some(true),
                ..Default::default()
            },
            &cancel,
        )
        .await
        .unwrap();
    let next = stream.next();
    tokio::pin!(next);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut next)
            .await
            .is_err()
    );
    cancel.cancel();
    let event = tokio::time::timeout(Duration::from_secs(1), next)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        event,
        Err(citadel_adapters::connectors::docker::DockerExecError::Cancelled)
    ));
    drop(stream);
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn terminal_input_half_close_drains_output_and_emits_one_exit() {
    use citadel_platforms::terminal::{TerminalInput, TerminalOutput};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let server = tokio::spawn(async move {
        for body in [
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            r#"{"Id":"exec-1"}"#,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            respond(&mut socket, body).await;
        }
        let (mut socket, _) = listener.accept().await.unwrap();
        request(&mut socket).await;
        socket
            .write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n",
            )
            .await
            .unwrap();
        let mut input = [0; 6];
        socket.read_exact(&mut input).await.unwrap();
        assert_eq!(&input, b"hello\x04");
        socket.write_all(b"last output").await.unwrap();
        socket.shutdown().await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(request(&mut socket).await.contains("/exec/exec-1/json"));
        respond(&mut socket, r#"{"Running":false,"ExitCode":3}"#).await;
    });
    let docker = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host").unwrap();
    let session = docker
        .execute_terminal(
            "test",
            &["cat".into()],
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
    session
        .input
        .try_send(TerminalInput::Stdin(b"hello".to_vec()))
        .unwrap();
    drop(session.input);
    let mut stream = session.output;
    let mut output = Vec::new();
    let mut exits = 0;
    tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(item) = stream.next().await {
            match item.unwrap() {
                TerminalOutput::Data(bytes) => output.extend(bytes),
                TerminalOutput::Exit(code) => {
                    assert_eq!(code, 3);
                    exits += 1;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(output, b"last output");
    assert_eq!(exits, 1);
    server.await.unwrap();
}
