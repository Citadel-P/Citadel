use super::*;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::get,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn sequential_checks_reuse_the_connection_and_read_fresh_responses() {
    use axum::extract::ConnectInfo;
    use std::{collections::HashSet, net::SocketAddr, sync::Mutex};
    let peers = Arc::new(Mutex::new(HashSet::new()));
    let observed = peers.clone();
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let app = Router::new().route("/", get(move |ConnectInfo(peer): ConnectInfo<SocketAddr>| {
        observed.lock().unwrap().insert(peer);
        let n = count.fetch_add(1, Ordering::SeqCst);
        async move {
            axum::Json(serde_json::json!({"tag_name":format!("v1.0.{n}"),"draft":false,"prerelease":false}))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let source = GithubReleaseSource::new().unwrap();
    for n in 0..3 {
        let releases = read_releases(&source.client, ReleaseChannel::Stable, &endpoint)
            .await
            .unwrap();
        assert_eq!(releases[0].tag_name, format!("v1.0.{n}"));
    }
    server.abort();
    let _ = server.await;
    assert_eq!(
        peers.lock().unwrap().len(),
        1,
        "checks should reuse one TCP connection"
    );
    assert_eq!(requests.load(Ordering::SeqCst), 3);
}
#[tokio::test]
async fn bounds_responses_rejects_http_failures_and_sends_no_instance_data() {
    let source = GithubReleaseSource::new().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let app = Router::new().route("/", get(move |headers: HeaderMap, uri: axum::http::Uri, body: bytes::Bytes| {
        let n = count.fetch_add(1, Ordering::SeqCst);
        async move {
            assert!(body.is_empty());
            assert!(uri.query().is_none());
            assert_eq!(headers["user-agent"], "Citadel-Update-Check");
            assert_eq!(headers["accept"], "application/vnd.github+json");
            assert!(!headers.contains_key("authorization"));
            assert!(!headers.contains_key("cookie"));
            match n {
                0 => Response::new(Body::from(r#"{"tag_name":"v1.1.0","draft":false,"prerelease":false,"html_url":"https://untrusted.invalid"}"#)),
                1 => Response::builder().status(StatusCode::TOO_MANY_REQUESTS).body(Body::empty()).unwrap(),
                2 => Response::new(Body::from("x".repeat(MAX_RESPONSE_BYTES + 1))),
                3 => Response::new(Body::from("invalid JSON")),
                4 => Response::builder().status(StatusCode::FOUND).header("location", "/").body(Body::empty()).unwrap(),
                5 => Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap(),
                6 => Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap(),
                _ => Response::new(Body::from(r#"{"tag_name":"v1.0.0","draft":false,"prerelease":false}"#)),
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let releases = read_releases(&source.client, ReleaseChannel::Stable, &endpoint)
        .await
        .unwrap();
    let update = citadel_updates::available_update("1.0.0", releases.into_iter().next().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        update.release_url,
        "https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0"
    );
    for _ in 0..6 {
        assert!(
            read_releases(&source.client, ReleaseChannel::Stable, &endpoint)
                .await
                .is_err()
        );
    }
    let releases = read_releases(&source.client, ReleaseChannel::Stable, &endpoint)
        .await
        .unwrap();
    assert!(
        citadel_updates::available_update("1.0.0", releases.into_iter().next().unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(requests.load(Ordering::SeqCst), 8);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn development_source_reads_release_lists_and_rejects_malformed_json() {
    let source = GithubReleaseSource::new().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let app = Router::new().route("/", get(move || {
        let n = count.fetch_add(1, Ordering::SeqCst);
        async move {
            match n {
                0 => r#"[{"tag_name":"v0.1.4","draft":false,"prerelease":false},{"tag_name":"v0.1.4-dev.2","draft":false,"prerelease":true}]"#,
                1 => "invalid JSON",
                _ => r#"[{"tag_name":"v0.1.4","draft":false,"prerelease":false},{"tag_name":"v0.1.4-dev.1","draft":false,"prerelease":true}]"#,
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let releases = read_releases(&source.client, ReleaseChannel::Development, &endpoint)
        .await
        .unwrap();
    assert_eq!(
        citadel_updates::development_update("0.1.4-dev.1", releases)
            .unwrap()
            .unwrap()
            .version,
        "0.1.4-dev.2"
    );
    assert!(
        read_releases(&source.client, ReleaseChannel::Development, &endpoint)
            .await
            .is_err()
    );
    let releases = read_releases(&source.client, ReleaseChannel::Development, &endpoint)
        .await
        .unwrap();
    assert!(
        citadel_updates::development_update("0.1.4-dev.1", releases)
            .unwrap()
            .is_none()
    );
    server.abort();
    let _ = server.await;
}
