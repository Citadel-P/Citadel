//! Executable probes against disposable listeners; shared by Core and Agent tests.
use std::{collections::BTreeMap, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
};

fn command(port: u16) -> Command {
    let mut command = Command::new(super::EXECUTABLE);
    command
        .env_clear()
        .envs(BTreeMap::from([
            ("CITADEL_TRANSPORT_MODE", "Disabled"),
            ("CITADEL_AGENT_MODE", "edge"),
            ("CITADEL_CORE_URL", "https://fixture.invalid"),
            ("CITADEL_EDGE_ENROLLMENT_TOKEN", "fixture-only"),
        ]))
        .env("CITADEL_HTTP_LISTEN_PORT", port.to_string())
        .env("CITADEL_AGENT_PORT", port.to_string())
        .arg("healthcheck")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}

async fn response(listener: &TcpListener, status: u16, body: &str, child: u32, stalled: bool) {
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut request = [0; 4096];
    let read = stream.read(&mut request).await.unwrap();
    assert!(request[..read].starts_with(b"GET /health "));
    #[cfg(target_os = "linux")]
    {
        let threads = std::fs::read_dir(format!("/proc/{child}/task"))
            .unwrap()
            .count();
        assert_eq!(
            threads, 1,
            "the HTTP probe must not create Tokio worker threads"
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = child;
    if stalled {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    let reply = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/health\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
}

#[tokio::test]
async fn automatic_probe_checks_listener_status_without_worker_threads() {
    for (status, body, healthy) in [
        (200, r#"{"Status":"Healthy"}"#, true),
        (503, r#"{"Status":"Unhealthy"}"#, false),
        (302, r#"{"Status":"Healthy"}"#, false),
        (204, "", false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut child = command(listener.local_addr().unwrap().port())
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        let (result, ()) =
            tokio::join!(child.wait(), response(&listener, status, body, pid, false));
        assert_eq!(result.unwrap().success(), healthy, "HTTP {status}");
    }
}

#[tokio::test]
async fn automatic_probe_bypasses_proxy_and_times_out_unresponsive_listeners() {
    let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut cmd = command(listener.local_addr().unwrap().port());
    for name in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        cmd.env(name, format!("http://{}", proxy.local_addr().unwrap()));
    }
    let mut child = cmd.spawn().unwrap();
    let pid = child.id().unwrap();
    let (result, ()) = tokio::join!(
        child.wait(),
        response(&listener, 200, r#"{"Status":"Healthy"}"#, pid, false)
    );
    assert!(result.unwrap().success());
    assert!(
        tokio::time::timeout(Duration::from_millis(50), proxy.accept())
            .await
            .is_err()
    );
    let mut child = command(listener.local_addr().unwrap().port())
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();
    let (result, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(2800), child.wait()),
        response(&listener, 200, r#"{"Status":"Healthy"}"#, pid, true)
    );
    assert!(
        !result
            .expect("probe deadline was extended")
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn absent_listener_fails() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    assert!(
        !tokio::time::timeout(Duration::from_secs(3), command(port).status())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
