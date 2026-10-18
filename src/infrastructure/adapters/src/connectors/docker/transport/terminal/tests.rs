use super::*;
use futures_util::StreamExt;
use std::process::Command;

#[test]
fn process_identity_rejects_init_missing_fields_and_malformed_ids() {
    let identity = parse_identity(b"CITADEL_TERMINAL 42 1234\r\n").unwrap();
    assert_eq!((identity.pid, identity.started), (42, 1234));
    for line in [
        "CITADEL_TERMINAL 1 1234",
        "CITADEL_TERMINAL 0 1234",
        "CITADEL_TERMINAL 42 0",
        "CITADEL_TERMINAL 42",
        "42 1234",
        "CITADEL_TERMINAL -42 1234",
        "CITADEL_TERMINAL 42 1234 extra",
    ] {
        assert!(parse_identity(line.as_bytes()).is_none(), "{line}");
    }
}

#[tokio::test]
async fn cancelled_open_waits_for_identity_then_cleans_up_the_started_exec() {
    for binary in [false, true] {
        cancelled_open(binary).await;
    }
}

async fn cancelled_open(binary: bool) {
    use tokio::net::{TcpListener, TcpStream};
    async fn request(socket: &mut TcpStream) -> (String, serde_json::Value) {
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            header.push(socket.read_u8().await.unwrap());
            assert!(header.len() < 16384);
        }
        let header = String::from_utf8(header).unwrap();
        let length = header
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        (
            header,
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
        )
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
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("tcp://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let tasks = citadel_runtime::DynamicTasks::new(CancellationToken::new());
    let client = DockerClient::with_endpoint(endpoint, Duration::from_secs(2), "/host")
        .unwrap()
        .with_terminal_tasks(tasks.clone());
    let (starting, started) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let server = tokio::spawn(async move {
        for body in [
            r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
            r#"{"Id":"exec-owned"}"#,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            request(&mut socket).await;
            respond(&mut socket, body).await;
        }
        let (mut terminal, _) = listener.accept().await.unwrap();
        let (header, _) = request(&mut terminal).await;
        assert!(header.starts_with("POST /v1.49/exec/exec-owned/start "));
        starting.send(()).unwrap();
        released.await.unwrap();
        terminal
            .write_all(b"HTTP/1.1 101 UPGRADED\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n")
            .await
            .unwrap();
        let identity = b"CITADEL_TERMINAL 42 1234\r\n";
        if binary {
            terminal.write_all(&[1, 0, 0, 0]).await.unwrap();
            terminal
                .write_all(&(identity.len() as u32).to_be_bytes())
                .await
                .unwrap();
        }
        terminal.write_all(identity).await.unwrap();
        let (mut inspect, _) = listener.accept().await.unwrap();
        assert!(
            request(&mut inspect)
                .await
                .0
                .starts_with("GET /v1.49/exec/exec-owned/json ")
        );
        respond(&mut inspect, r#"{"Running":true}"#).await;
        let (mut create, _) = listener.accept().await.unwrap();
        let (header, body) = request(&mut create).await;
        assert!(header.starts_with("POST /v1.49/containers/owned-container/exec "));
        assert_eq!(body["Cmd"][4], "42");
        assert_eq!(body["Cmd"][5], "1234");
        assert_eq!(body["Cmd"][6], "cleanup");
        assert!(body.get("Privileged").is_none());
        respond(&mut create, r#"{"Id":"cleanup"}"#).await;
        let (mut cleanup, _) = listener.accept().await.unwrap();
        assert!(
            request(&mut cleanup)
                .await
                .0
                .starts_with("POST /v1.49/exec/cleanup/start ")
        );
        cleanup
            .write_all(b"HTTP/1.1 101 UPGRADED\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n")
            .await
            .unwrap();
        cleanup.shutdown().await.unwrap();
        let (mut control, _) = listener.accept().await.unwrap();
        assert!(request(&mut control).await.0.contains("/exec/cleanup/json"));
        respond(&mut control, r#"{"Running":false,"ExitCode":0}"#).await;
        let (mut inspect, _) = listener.accept().await.unwrap();
        assert!(
            request(&mut inspect)
                .await
                .0
                .starts_with("GET /v1.49/exec/exec-owned/json ")
        );
        respond(&mut inspect, r#"{"Running":false,"ExitCode":137}"#).await;
        let mut byte = [0];
        assert_eq!(terminal.read(&mut byte).await.unwrap(), 0);
    });
    let cancel = CancellationToken::new();
    let driver_cancel = cancel.clone();
    let opening = tokio::spawn(async move {
        if binary {
            client
                .execute_binary(
                    "owned-container",
                    citadel_docker_api::models::ExecConfig {
                        cmd: Some(vec!["/bin/sh".into()]),
                        attach_stdout: Some(true),
                        ..Default::default()
                    },
                    &driver_cancel,
                )
                .await
                .map(|_| ())
        } else {
            client
                .execute_terminal("owned-container", &["/bin/sh".into()], &driver_cancel)
                .await
                .map(|_| ())
                .map_err(crate::connectors::docker::runtime::normalize_docker_error)
        }
    });
    timeout(Duration::from_secs(2), started)
        .await
        .unwrap()
        .unwrap();
    cancel.cancel();
    assert!(opening.await.unwrap().is_err());
    release.send(()).unwrap();
    tasks.drain(Duration::from_secs(5)).await.unwrap();
    timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
}

struct Fixture {
    name: String,
    client: DockerClient,
    tasks: citadel_runtime::DynamicTasks,
    shutdown: CancellationToken,
}

impl Fixture {
    fn new() -> Self {
        Self::with_init(true)
    }

    fn with_init(init: bool) -> Self {
        let name = format!("citadel-terminal-test-{}", uuid::Uuid::now_v7());
        let shutdown = CancellationToken::new();
        let tasks = citadel_runtime::DynamicTasks::new(shutdown.clone());
        let client = DockerClient::new("/var/run/docker.sock", Duration::from_secs(2))
            .unwrap()
            .with_terminal_tasks(tasks.clone());
        let fixture = Self {
            name,
            client,
            tasks,
            shutdown,
        };
        let mut args = vec!["run", "--detach", "--rm"];
        if init {
            args.push("--init");
        }
        args.extend([
            "--name",
            &fixture.name,
            "--entrypoint",
            "/bin/sh",
            "redis:latest",
            "-c",
            "exec sleep 300",
        ]);
        fixture.docker(&args);
        fixture
    }

    fn docker(&self, args: &[&str]) -> String {
        let output = Command::new("docker").args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    async fn open(&self, command: &[&str], idle: Duration) -> TerminalSession {
        self.client
            .execute_terminal_with_idle(
                &self.name,
                &command.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                &CancellationToken::new(),
                idle,
            )
            .await
            .unwrap()
    }

    fn exec_ids(&self) -> Vec<String> {
        serde_json::from_str::<Option<Vec<String>>>(&self.docker(&[
            "inspect",
            &self.name,
            "--format",
            "{{json .ExecIDs}}",
        ]))
        .unwrap()
        .unwrap_or_default()
    }

    async fn drained(&self) {
        self.tasks.drain(Duration::from_secs(8)).await.unwrap();
        assert!(self.exec_ids().is_empty(), "terminal exec survived cleanup");
        assert_eq!(
            self.docker(&[
                "inspect",
                &self.name,
                "--format",
                "{{.State.Running}} {{.RestartCount}} "
            ])
            .trim(),
            "true 0"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.shutdown.cancel();
        let _ = Command::new("docker")
            .args(["rm", "--force", &self.name])
            .output();
    }
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn dropping_unpolled_terminal_output_terminates_the_shell() {
    let f = Fixture::new();
    let session = f.open(&["/bin/bash"], IDLE_TIMEOUT).await;
    assert_eq!(f.exec_ids().len(), 1);
    drop(session.output);
    f.drained().await;
    assert!(session.input.is_closed());
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn cancellation_terminates_foreground_jobs_but_not_another_terminal() {
    let f = Fixture::new();
    let mut first = f
        .open(
            &[
                "/bin/sh",
                "-c",
                "trap '' HUP TERM; sleep 300 & echo ready; wait",
            ],
            IDLE_TIMEOUT,
        )
        .await;
    while !matches!(first.output.next().await.unwrap().unwrap(), TerminalOutput::Data(bytes) if String::from_utf8_lossy(&bytes).contains("ready"))
    {
    }
    let mut other = f.open(&["/bin/bash"], IDLE_TIMEOUT).await;
    drop(first);
    timeout(Duration::from_secs(5), async {
        while f.tasks.active() != 1 {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let processes = f.docker(&["top", &f.name, "-eo", "pid,comm"]);
    assert_eq!(
        processes
            .lines()
            .filter(|line| line.split_whitespace().last() == Some("sleep"))
            .count(),
        1,
        "foreground child survived: {processes}"
    );
    other
        .input
        .try_send(TerminalInput::Stdin(b"printf 'still-alive\\n'\n".to_vec()))
        .unwrap();
    timeout(Duration::from_secs(2), async {
        while !matches!(other.output.next().await.unwrap().unwrap(), TerminalOutput::Data(bytes) if String::from_utf8_lossy(&bytes).contains("still-alive")) {}
    }).await.unwrap();
    drop(other);
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn inactivity_closes_the_session_without_output_being_polled() {
    let f = Fixture::new();
    let mut session = f
        .open(&["/bin/sh", "-c", "read value"], Duration::from_millis(200))
        .await;
    f.drained().await;
    let mut bytes = Vec::new();
    while let Some(item) = session.output.next().await {
        if let TerminalOutput::Data(data) = item.unwrap() {
            bytes.extend(data);
        }
    }
    assert!(String::from_utf8_lossy(&bytes).contains("disconnected after inactivity"));
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn shutdown_cleans_up_a_retained_terminal() {
    let f = Fixture::new();
    let session = f.open(&["/bin/bash"], IDLE_TIMEOUT).await;
    f.shutdown.cancel();
    f.drained().await;
    assert!(session.input.is_closed());
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn cleanup_refuses_a_reused_pid_or_another_process_identity() {
    let f = Fixture::new();
    let (socket, id, identity) = f
        .client
        .open_owned_terminal(&f.name, &["/bin/bash".into()])
        .await
        .unwrap();
    assert!(
        timeout(
            CLEANUP_TIMEOUT,
            f.client.cleanup_exec(
                &f.name,
                &id,
                &ProcessIdentity {
                    pid: identity.pid,
                    started: identity.started + 1
                },
                None
            )
        )
        .await
        .is_ok_and(|result| result.is_err())
    );
    assert_eq!(
        f.client.exec_inspect(&id).await.unwrap().running,
        Some(true)
    );
    f.client
        .cleanup_exec(&f.name, &id, &identity, None)
        .await
        .unwrap();
    drop(socket);
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn closed_stdin_drains_output_and_preserves_exit_status() {
    let f = Fixture::new();
    let mut session = f
        .open(
            &[
                "/bin/sh",
                "-c",
                "read value; printf 'reply:%s' \"$value\"; exit 4",
            ],
            IDLE_TIMEOUT,
        )
        .await;
    session
        .input
        .try_send(TerminalInput::Stdin(b"hello\n".to_vec()))
        .unwrap();
    drop(session.input);
    let mut bytes = Vec::new();
    let mut exit = None;
    while let Some(item) = session.output.next().await {
        match item.unwrap() {
            TerminalOutput::Data(data) => bytes.extend(data),
            TerminalOutput::Exit(code) => exit = Some(code),
        }
    }
    assert!(String::from_utf8_lossy(&bytes).contains("reply:hello"));
    assert_eq!(exit, Some(4));
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn terminal_output_extends_idle_deadline() {
    let f = Fixture::new();
    let mut session = f
        .open(
            &[
                "/bin/sh",
                "-c",
                "echo tick; sleep 0.6; echo tick; sleep 0.6; echo tick; read value",
            ],
            Duration::from_secs(1),
        )
        .await;
    let mut bytes = Vec::new();
    timeout(Duration::from_secs(5), async {
        while let Some(item) = session.output.next().await {
            if let TerminalOutput::Data(data) = item.unwrap() {
                bytes.extend(data);
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&bytes).matches("tick").count(), 3);
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn resize_does_not_keep_an_idle_terminal_alive() {
    let f = Fixture::new();
    let session = f
        .open(&["/bin/sh", "-c", "read value"], Duration::from_millis(500))
        .await;
    timeout(Duration::from_secs(2), async {
        while !session.input.is_closed() {
            let _ = session
                .input
                .try_send(TerminalInput::Resize { cols: 80, rows: 24 });
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn request_cancellation_cleans_up_even_if_output_is_retained() {
    let f = Fixture::new();
    let cancel = CancellationToken::new();
    let session = f
        .client
        .execute_terminal(&f.name, &["/bin/bash".into()], &cancel)
        .await
        .unwrap();
    cancel.cancel();
    f.drained().await;
    assert!(session.input.is_closed());
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn binary_drop_and_cancellation_clean_up_without_polling_output() {
    use citadel_docker_api::models::ExecConfig;
    for cancel_request in [false, true] {
        let f = Fixture::new();
        let cancel = CancellationToken::new();
        let stream = f
            .client
            .execute_binary(
                &f.name,
                ExecConfig {
                    cmd: Some(vec![
                        "/bin/sh".into(),
                        "-c".into(),
                        "sleep 300 & wait".into(),
                    ]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    user: Some("65534".into()),
                    ..Default::default()
                },
                &cancel,
            )
            .await
            .unwrap();
        assert_eq!(f.exec_ids().len(), 1);
        if cancel_request {
            cancel.cancel();
            f.drained().await;
            drop(stream);
        } else {
            drop(stream);
            f.drained().await;
        }
        let processes = f.docker(&["top", &f.name, "-eo", "pid,comm"]);
        assert_eq!(
            processes
                .lines()
                .filter(|line| line.split_whitespace().last() == Some("sleep"))
                .count(),
            1,
            "binary exec child survived: {processes}"
        );
    }
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn binary_backpressure_and_shutdown_cleanup_preserve_other_sessions() {
    use citadel_docker_api::models::ExecConfig;
    let f = Fixture::new();
    let cancel = CancellationToken::new();
    let other = f.open(&["/bin/bash"], IDLE_TIMEOUT).await;
    let stream = f
        .client
        .execute_binary(
            &f.name,
            ExecConfig {
                cmd: Some(vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "while :; do printf 'output-output-output'; done".into(),
                ]),
                attach_stdout: Some(true),
                ..Default::default()
            },
            &cancel,
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    cancel.cancel();
    timeout(Duration::from_secs(5), async {
        while f.tasks.active() != 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(!other.input.is_closed());
    drop(stream);
    let retained = f
        .client
        .execute_binary(
            &f.name,
            ExecConfig {
                cmd: Some(vec!["sleep".into(), "300".into()]),
                attach_stdout: Some(true),
                ..Default::default()
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    f.shutdown.cancel();
    f.drained().await;
    drop((other, retained));
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn binary_execution_preserves_bytes_options_and_exit_without_identity_output() {
    use super::super::DockerExecEvent;
    use citadel_docker_api::models::ExecConfig;
    for stdout_enabled in [false, true] {
        let f = Fixture::new();
        let mut stream = f
            .client
            .execute_binary(
                &f.name,
                ExecConfig {
                    cmd: Some(vec![
                        "/bin/sh".into(),
                        "-c".into(),
                        "printf '\\000\\377A'; printf '%s:%s' \"$MODE\" \"$PWD\" >&2; exit 7"
                            .into(),
                    ]),
                    env: Some(vec!["MODE=fixture".into()]),
                    working_dir: Some("/tmp".into()),
                    attach_stdout: Some(stdout_enabled),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        let (mut stdout, mut stderr, mut exits) = (Vec::new(), Vec::new(), Vec::new());
        while let Some(item) = stream.next().await {
            match item.unwrap() {
                DockerExecEvent::Output {
                    data,
                    stderr: false,
                } => stdout.extend(data),
                DockerExecEvent::Output { data, stderr: true } => stderr.extend(data),
                DockerExecEvent::Exit(code) => exits.push(code),
            }
        }
        assert_eq!(
            stdout,
            if stdout_enabled {
                vec![0, 255, b'A']
            } else {
                vec![]
            }
        );
        assert_eq!(stderr, b"fixture:/tmp");
        assert_eq!(exits, [7]);
        f.drained().await;
    }
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn output_burst_waits_for_consumer_without_truncating_or_killing_terminal() {
    let f = Fixture::new();
    let mut session = f
        .open(
            &[
                "/bin/sh",
                "-c",
                "head -c 1048576 /dev/zero; read value; exit 7",
            ],
            IDLE_TIMEOUT,
        )
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !session.input.is_closed(),
        "output backpressure killed the terminal"
    );
    let mut bytes = Vec::new();
    timeout(Duration::from_secs(5), async {
        while bytes.len() < 1048576 {
            match session.output.next().await.unwrap().unwrap() {
                TerminalOutput::Data(data) => bytes.extend(data),
                TerminalOutput::Exit(_) => panic!("terminal exited before input"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(bytes, vec![0; 1048576]);
    session
        .input
        .try_send(TerminalInput::Stdin(b"done\n".to_vec()))
        .unwrap();
    let mut exits = Vec::new();
    timeout(Duration::from_secs(5), async {
        while let Some(item) = session.output.next().await {
            if let TerminalOutput::Exit(code) = item.unwrap() {
                exits.push(code);
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(exits, [7]);
    f.drained().await;
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn cancellation_interrupts_backpressured_terminal_output() {
    let f = Fixture::new();
    let cancel = CancellationToken::new();
    let session = f
        .client
        .execute_terminal(
            &f.name,
            &[
                "/bin/sh".into(),
                "-c".into(),
                "while :; do printf output; done".into(),
            ],
            &cancel,
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!session.input.is_closed());
    cancel.cancel();
    f.drained().await;
    assert!(session.input.is_closed());
}

#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only a disposable container"]
async fn cleanup_reaches_descendants_when_killing_a_child_exits_the_leader() {
    let f = Fixture::new();
    let mut stream = f
        .client
        .execute_binary(
            &f.name,
            citadel_docker_api::models::ExecConfig {
                cmd: Some(vec![
                    "/bin/bash".into(),
                    "-c".into(),
                    "(for i in $(seq 1 20); do sleep 299 & done; echo ready; wait) & wait".into(),
                ]),
                attach_stdout: Some(true),
                ..Default::default()
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let mut bytes = Vec::new();
    timeout(Duration::from_secs(5), async {
        while !String::from_utf8_lossy(&bytes).contains("ready") {
            if let super::super::DockerExecEvent::Output { data, .. } =
                stream.next().await.unwrap().unwrap()
            {
                bytes.extend(data);
            }
        }
    })
    .await
    .unwrap();
    drop(stream);
    f.drained().await;
    timeout(Duration::from_secs(3), async {
        loop {
            let processes = f.docker(&["top", &f.name, "-eo", "pid,comm"]);
            if processes
                .lines()
                .filter(|line| line.split_whitespace().last() == Some("sleep"))
                .count()
                == 1
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("orphaned grandchildren survived cleanup");
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires local Docker and redis:latest; creates only disposable containers"]
async fn startup_and_approval_timeouts_release_exec_without_losing_ownership() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::net::{TcpListener, UnixStream};
    for (binary, delay_start) in [(false, 1), (true, 1), (false, 2), (true, 2)] {
        let f = Fixture::with_init(delay_start != 1);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = DockerClient::with_endpoint(
            format!("tcp://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(20),
            "/host",
        )
        .unwrap()
        .with_terminal_tasks(f.tasks.clone());
        let proxy = tokio_util::task::AbortOnDropHandle::new(tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            let starts = Arc::new(AtomicUsize::new(0));
            loop {
                tokio::select! {
                    Some(result) = connections.join_next(), if !connections.is_empty() => { result.unwrap(); },
                    connection = listener.accept() => {
                        let (mut browser, _) = connection.unwrap();
                        let starts = starts.clone();
                        connections.spawn(async move {
                            let mut docker = UnixStream::connect("/var/run/docker.sock").await.unwrap();
                            let mut header = Vec::new();
                            while !header.ends_with(b"\r\n\r\n") {
                                header.push(browser.read_u8().await.unwrap());
                            }
                            let text = String::from_utf8(header.clone()).unwrap();
                            let length = text.lines().find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().unwrap())
                            }).unwrap_or(0);
                            let mut body = vec![0; length];
                            browser.read_exact(&mut body).await.unwrap();
                            let start = text.lines().next().unwrap().contains("/start ");
                            // A persistent /version connection would otherwise carry
                            // later requests past this request-level delay fixture.
                            if !start {
                                header.truncate(header.len() - 2);
                                header.extend_from_slice(b"Connection: close\r\n\r\n");
                            }
                            docker.write_all(&header).await.unwrap();
                            docker.write_all(&body).await.unwrap();
                            // Delay either the launch response or the approval response
                            // after Docker has accepted that mutation.
                            if start && starts.fetch_add(1, Ordering::SeqCst) + 1 == delay_start {
                                tokio::time::sleep(Duration::from_secs(6)).await;
                            }
                            let _ = tokio::io::copy_bidirectional(&mut browser, &mut docker).await;
                        });
                    }
                }
            }
        }));
        let command = vec![
            "/bin/sh".into(),
            "-c".into(),
            "touch /tmp/must-not-run; sleep 300".into(),
        ];
        let failed = if binary {
            client
                .execute_binary(
                    &f.name,
                    citadel_docker_api::models::ExecConfig {
                        cmd: Some(command),
                        attach_stdout: Some(true),
                        ..Default::default()
                    },
                    &CancellationToken::new(),
                )
                .await
                .is_err()
        } else {
            client
                .execute_terminal(&f.name, &command, &CancellationToken::new())
                .await
                .is_err()
        };
        assert!(
            failed,
            "delayed startup unexpectedly succeeded: binary={binary}, delay_start={delay_start}"
        );
        timeout(Duration::from_secs(12), async {
            while !f.exec_ids().is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("unapproved launcher survived its watchdog deadline");
        if delay_start == 1 {
            f.docker(&["exec", &f.name, "test", "!", "-e", "/tmp/must-not-run"]);
            let processes = f.docker(&["top", &f.name, "-eo", "pid,comm"]);
            assert_eq!(
                processes.lines().count(),
                2,
                "startup orphaned a watchdog in a container without init: {processes}"
            );
        } else {
            // Approval reached Docker, so execution was permitted, but the owned
            // cleanup must still stop it when the approval response is lost.
            f.docker(&["exec", &f.name, "test", "-e", "/tmp/must-not-run"]);
            let processes = f.docker(&["top", &f.name, "-eo", "pid,comm"]);
            assert_eq!(
                processes
                    .lines()
                    .filter(|line| line.split_whitespace().last() == Some("sleep"))
                    .count(),
                1
            );
        }
        f.drained().await;
        drop(proxy);
    }
}
