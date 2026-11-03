use super::admit;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
    routing::{get, post},
};
use citadel_runtime::admission::{AdmissionError, AdmissionGate};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn closed_admission_blocks_handlers_but_preserves_status_and_liveness() {
    let gate = AdmissionGate::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let app = Router::new()
        .route(
            "/mutate",
            post(move || async move {
                observed.fetch_add(1, Ordering::SeqCst);
                "changed"
            }),
        )
        .route(
            "/api/v1/updates",
            get(|| async { StatusCode::UNAUTHORIZED }),
        )
        .route(
            "/api/v1/updates/operations/{id}",
            get(|| async { StatusCode::UNAUTHORIZED }),
        )
        .route("/health", get(|| async { "alive" }))
        .route("/ready", get(|| async { "ready" }))
        .layer(middleware::from_fn_with_state(gate.clone(), admit));
    let fence = Uuid::now_v7();
    gate.close(fence).unwrap();
    for path in ["/mutate", "/ready"] {
        let result = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(if path == "/mutate" { "POST" } else { "GET" })
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(result.headers()["retry-after"], "5");
        assert_eq!(result.headers()["cache-control"], "no-store");
        let bytes = to_bytes(result.into_body(), 4096).await.unwrap();
        let problem: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(problem["type"], "maintenance");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for (path, status) in [
        ("/health", StatusCode::OK),
        ("/api/v1/updates", StatusCode::UNAUTHORIZED),
        (
            "/api/v1/updates/operations/019a0000-0000-7000-8000-000000000001",
            StatusCode::UNAUTHORIZED,
        ),
        (
            "/api/v1/updates/operations/not-an-id",
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            status
        );
    }
    gate.reopen(fence).unwrap();
    assert_eq!(
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/mutate")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::OK
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn streamed_work_remains_admitted_after_headers_and_is_released_on_disconnect() {
    let gate = AdmissionGate::default();
    let app = Router::new()
        .route(
            "/stream",
            post(|| async {
                Body::from_stream(futures_util::stream::pending::<
                    Result<bytes::Bytes, std::io::Error>,
                >())
            }),
        )
        .layer(middleware::from_fn_with_state(gate.clone(), admit));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let fence = Uuid::now_v7();
    gate.close(fence).unwrap();
    assert!(matches!(
        gate.drain(fence, Duration::from_millis(1), &CancellationToken::new())
            .await,
        Err(AdmissionError::Busy(_))
    ));
    drop(response);
    gate.drain(fence, Duration::from_secs(1), &CancellationToken::new())
        .await
        .unwrap();
    assert!(gate.is_closed());
}
