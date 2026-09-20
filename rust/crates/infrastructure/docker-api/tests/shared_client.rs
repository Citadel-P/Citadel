#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use citadel_docker_api::apis::{
    configuration::Configuration,
    distribution_api::{DistributionApi, DistributionApiClient},
    system_api::{SystemApi, SystemApiClient},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct SocketPath(std::path::PathBuf);
impl Drop for SocketPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::test]
async fn tag_clients_share_the_injected_unix_connection_and_registry_auth() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let socket = SocketPath(std::env::temp_dir().join(format!("citadel-api-{}.sock", std::process::id())));
        let listener = tokio::net::UnixListener::bind(&socket.0).unwrap();
        let server = tokio::spawn(async move {
            // Accept exactly once: all three calls must reuse the supplied client's pool.
            let (mut stream, _) = listener.accept().await.unwrap();
            for (path, content_type, body) in [
                ("/v1.49/version", "application/json", r#"{"Version":"28.0.0","ApiVersion":"1.49","Future":true}"#),
                ("/v1.49/distribution/alpine/json", "application/json", r#"{"Descriptor":{"digest":"sha256:test"},"Platforms":[]}"#),
                ("/v1.49/_ping", "text/plain", "OK"),
            ] {
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let byte = stream.read_u8().await.unwrap();
                    request.push(byte);
                    assert!(request.len() < 16_384);
                }
                let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
                assert!(request.starts_with(&format!("get {path} http/1.1\r\n")), "{request}");
                assert!(request.contains("x-citadel-fixture: shared-client\r\n"));
                if path.contains("distribution") {
                    assert!(request.contains("x-registry-auth: registry-token\r\n"));
                }
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-citadel-fixture", "shared-client".parse().unwrap());
        let client = reqwest::Client::builder().unix_socket(socket.0.as_path())
            .default_headers(headers).build().unwrap();
        // Explicit construction avoids Configuration::default() and its independent pool.
        let configuration = Arc::new(Configuration {
            request_timeout: Some(Duration::from_secs(5)),
            max_response_bytes: 16 * 1024 * 1024,
            max_error_bytes: 64 * 1024,
            base_path: "http://localhost/v1.49".into(),
            client,
            user_agent: None,
            basic_auth: None,
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: None,
        });
        let system = SystemApiClient::new(configuration.clone());
        let distribution = DistributionApiClient::new(configuration.clone());
        assert_eq!(Arc::strong_count(&configuration), 3);
        let version = system.system_version().await.unwrap();
        assert_eq!(version.api_version.as_deref(), Some("1.49"));
        let image = distribution.distribution_inspect("alpine", Some("registry-token")).await.unwrap();
        assert_eq!(image.descriptor.digest.as_deref(), Some("sha256:test"));
        assert_eq!(system.system_ping().await.unwrap(), "OK");
        server.await.unwrap();
    }).await.expect("generated tag clients must reuse the Unix connection");
}
