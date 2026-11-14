const EXECUTABLE: &str = env!("CARGO_BIN_EXE_citadel-server");
#[path = "../../../test/rust/healthcheck.rs"]
mod probes;

#[tokio::test]
async fn custom_url_preserves_redirect_support() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for (path, reply) in [
            (
                "/custom",
                "HTTP/1.1 302 Found\r\nLocation: /success\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ),
            (
                "/success",
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let n = socket.read(&mut request).await.unwrap();
            assert!(request[..n].starts_with(format!("GET {path} ").as_bytes()));
            socket.write_all(reply.as_bytes()).await.unwrap();
        }
    });
    let output = tokio::process::Command::new(EXECUTABLE)
        .env_clear()
        .args(["healthcheck", "--url", &format!("http://{address}/custom")])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn direct_tls_probe_trusts_only_the_configured_chain() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let directory =
        std::env::temp_dir().join(format!("citadel-health-tls-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&directory).unwrap();
    for name in ["trusted", "other"] {
        let status = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=core.fixture.invalid",
                "-addext",
                "subjectAltName=DNS:core.fixture.invalid",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-keyout",
                &format!("{name}.key"),
                "-out",
                &format!("{name}.pem"),
            ])
            .current_dir(&directory)
            .output()
            .unwrap();
        assert!(status.status.success());
    }
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(
        directory.join("trusted.pem"),
        directory.join("trusted.key"),
    )
    .await
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let handle = axum_server::Handle::new();
    let server = tokio::spawn(
        axum_server::from_tcp_rustls(listener, tls)
            .unwrap()
            .handle(handle.clone())
            .serve(
                axum::Router::new()
                    .route("/health", axum::routing::get(|| async { "healthy" }))
                    .into_make_service(),
            ),
    );
    for (certificate, healthy) in [("trusted.pem", true), ("other.pem", false)] {
        let output = tokio::process::Command::new(EXECUTABLE)
            .env_clear()
            .env("CITADEL_TRANSPORT_MODE", "Direct")
            .env("CITADEL_HTTP_LISTEN_PORT", address.port().to_string())
            .env("CITADEL_TLS_CERTIFICATE_PATH", directory.join(certificate))
            .arg("healthcheck")
            .output()
            .await
            .unwrap();
        assert_eq!(
            output.status.success(),
            healthy,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    handle.shutdown();
    server.await.unwrap().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
