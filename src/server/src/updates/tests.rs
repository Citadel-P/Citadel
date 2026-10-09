use super::*;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::get,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn release(tag: &str) -> Release {
    Release {
        tag_name: tag.into(),
        draft: false,
        prerelease: false,
    }
}

#[test]
fn compares_semantic_versions_and_ignores_build_metadata() {
    for (current, tag, expected) in [
        ("1.0.0", "v1.1.0", true),
        ("1.9.0", "v1.10.0", true),
        ("1.1.0", "v1.1.0", false),
        ("1.2.0", "v1.1.0", false),
        ("1.1.0+local", "v1.1.0+release", false),
        ("1.1.0-dev.1", "v1.1.0", true),
        ("1.2.0-dev.1", "v1.1.0", false),
        ("1.0.0", "v1.1.0-dev.1", false),
    ] {
        assert_eq!(
            available_update(current, release(tag)).unwrap().is_some(),
            expected,
            "{current} -> {tag}"
        );
    }
    let update = available_update("1.0.0", release("v1.1.0"))
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "1.1.0");
    assert_eq!(
        update.release_url,
        "https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0"
    );
}

#[test]
fn rejects_drafts_prereleases_and_invalid_tags() {
    let mut draft = release("v2.0.0");
    draft.draft = true;
    assert!(available_update("1.0.0", draft).unwrap().is_none());
    let mut preview = release("v2.0.0");
    preview.prerelease = true;
    assert!(available_update("1.0.0", preview).unwrap().is_none());
    for tag in ["latest", "v2.0.0/../../evil", "2.0", "<script>"] {
        assert!(available_update("1.0.0", release(tag)).is_err());
    }
    assert!(available_update("unknown", release("v2.0.0")).is_err());
}

#[tokio::test(start_paused = true)]
async fn disabled_worker_stays_idle_until_cancelled() {
    let checker = UpdateChecker::default();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(checker.clone().run(false, cancellation.clone()));
    tokio::time::advance(CHECK_INTERVAL * 2).await;
    assert!(!task.is_finished());
    assert!(checker.available().await.is_none());
    cancellation.cancel();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn bounds_responses_preserves_cache_on_failure_and_sends_no_instance_data() {
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
    let checker = UpdateChecker::default();
    checker.check(&endpoint, "1.0.0").await;
    let update = checker.available().await.unwrap();
    assert_eq!(
        update.release_url,
        "https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0"
    );
    for _ in 0..6 {
        checker.check(&endpoint, "1.0.0").await;
        assert_eq!(checker.available().await, Some(update.clone()));
    }
    checker.check(&endpoint, "1.0.0").await;
    assert!(checker.available().await.is_none());
    assert_eq!(requests.load(Ordering::SeqCst), 8);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn application_info_only_exposes_updates_to_administrators() {
    use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType};
    use tower::ServiceExt;
    let checker = UpdateChecker::default();
    *checker.0.write().await = available_update("1.0.0", release("v1.1.0")).unwrap();
    for (principal_type, roles, expected) in [
        (
            AuthenticatedPrincipalType::User,
            vec!["Admin".to_owned()],
            true,
        ),
        (AuthenticatedPrincipalType::User, vec![], false),
        (
            AuthenticatedPrincipalType::ServiceAccount,
            vec!["Admin".to_owned()],
            false,
        ),
    ] {
        let principal = ActorPrincipal {
            subject_id: uuid::Uuid::nil(),
            actor_id: citadel_primitives::ActorId::new(uuid::Uuid::nil()),
            name: "test".into(),
            principal_type,
            credential_id: None,
            roles,
        };
        let response = crate::application_info_http::router(checker.clone())
            .layer(axum::Extension(principal))
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/application/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.get("availableUpdate").is_some(), expected);
    }
}
