use crate::{Readiness, metrics::Metrics, openapi::router::OpenApiRouterExt};
use axum::{
    Extension, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use citadel_docker::client::DockerClient;
use citadel_runtime::database_health::DatabaseHealth;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_READINESS_PROBES: usize = 4;

#[derive(Clone)]
struct ReadinessProbes(Arc<Semaphore>);

#[derive(Clone)]
pub struct DiagnosticsHttpState {
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub database: Arc<dyn DatabaseHealth>,
    pub docker: DockerClient,
}

#[derive(Serialize, utoipa::ToSchema)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: DiagnosticsHttpState) -> Router {
    documented_routes()
        .split_for_parts()
        .0
        .with_state(state)
        .layer(Extension(ReadinessProbes(Arc::new(Semaphore::new(
            MAX_READINESS_PROBES,
        )))))
}

/// Restricted startup listener: no database or Docker probes can make an
/// unresolved update ready, and no application routes are constructed.
pub fn maintenance_router() -> Router {
    Router::new()
        .route("/health", axum::routing::get(health))
        .route(
            "/ready",
            axum::routing::get(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    axum::Json(Readiness::default().snapshot()),
                )
            }),
        )
        .fallback(|| async { StatusCode::SERVICE_UNAVAILABLE })
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "getHealth",
    tag = "Diagnostics",
    summary = "Process liveness",
    responses(
        (status = 200, description = "Success", body = HealthResponse, content_type = "application/json"),
        crate::openapi::errors::RequestErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn health() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/ready",
    operation_id = "getReadiness",
    tag = "Diagnostics",
    summary = "Dependency readiness",
    responses(
        (status = 200, description = "Success", body = crate::ReadinessResponse, content_type = "application/json"),
        crate::openapi::errors::ReadinessErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn ready(
    State(state): State<DiagnosticsHttpState>,
    Extension(probes): Extension<ReadinessProbes>,
) -> Response {
    // Never queue unbounded dependency checks behind a slow database or daemon.
    let Ok(_permit) = probes.0.try_acquire() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::Readiness.start();
    let (database, docker) = tokio::join!(
        tokio::time::timeout(std::time::Duration::from_secs(2), state.database.healthy()),
        tokio::time::timeout(std::time::Duration::from_secs(2), state.docker.ping()),
    );
    let database = matches!(database, Ok(true));
    let docker = matches!(docker, Ok(Ok(_)));
    if !database || !docker {
        state.metrics.readiness_failed();
    }
    let mut readiness = state.readiness.snapshot();
    let is_ready = database && docker;
    readiness.database = database;
    readiness.docker = docker;
    readiness.status = if is_ready { "ready" } else { "not-ready" };
    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, axum::Json(readiness)).into_response()
}

#[utoipa::path(
    get,
    path = "/metrics",
    operation_id = "getMetrics",
    tag = "Diagnostics",
    summary = "OpenMetrics diagnostics",
    responses(
        (status = 200, description = "Success"),
        crate::openapi::errors::RequestErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn open_metrics(State(state): State<DiagnosticsHttpState>) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        state.metrics.render(state.database.pool_usage()),
    )
        .into_response()
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DiagnosticsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(health))
        .normalized_routes(utoipa_axum::routes!(ready))
        .normalized_routes(utoipa_axum::routes!(open_metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use citadel_runtime::database_health::DatabasePoolUsage;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tower::ServiceExt;

    #[tokio::test]
    async fn maintenance_never_exposes_application_routes_or_dependency_readiness() {
        let app = maintenance_router();
        for (method, path, status) in [
            ("GET", "/health", StatusCode::OK),
            ("GET", "/ready", StatusCode::SERVICE_UNAVAILABLE),
            (
                "POST",
                "/api/v1/setup/initialize",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            ("GET", "/api/v1/updates", StatusCode::SERVICE_UNAVAILABLE),
            ("GET", "/metrics", StatusCode::SERVICE_UNAVAILABLE),
            ("GET", "/realtime", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{method} {path}");
            if path == "/ready" {
                let bytes = axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap();
                let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(body["status"], "not-ready");
                assert_eq!(body["database"], false);
                assert_eq!(body["docker"], false);
                assert_eq!(body["setup"], false);
            }
        }
    }

    struct BlockingDatabase {
        calls: AtomicUsize,
        release: Semaphore,
    }
    impl DatabaseHealth for BlockingDatabase {
        fn healthy(&self) -> futures_util::future::BoxFuture<'_, bool> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.release.acquire().await.unwrap().forget();
                true
            })
        }
        fn pool_usage(&self) -> DatabasePoolUsage {
            DatabasePoolUsage {
                connections: 0,
                idle: 0,
            }
        }
    }

    #[tokio::test]
    async fn readiness_probes_are_bounded_without_blocking_liveness_and_cancel_releases_capacity() {
        let database = Arc::new(BlockingDatabase {
            calls: AtomicUsize::new(0),
            release: Semaphore::new(0),
        });
        let app = router(DiagnosticsHttpState {
            readiness: Arc::new(Readiness::default()),
            metrics: Arc::new(Metrics::default()),
            database: database.clone(),
            docker: DockerClient::new(
                "/nonexistent-citadel-readiness-test.sock",
                Duration::from_millis(10),
            )
            .unwrap(),
        });
        let request = |path| Request::builder().uri(path).body(Body::empty()).unwrap();
        let mut tasks = Vec::new();
        for _ in 0..MAX_READINESS_PROBES {
            tasks.push(tokio::spawn(app.clone().oneshot(request("/ready"))));
        }
        let wait_calls = async |expected| {
            tokio::time::timeout(Duration::from_secs(1), async {
                while database.calls.load(Ordering::SeqCst) < expected {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        };
        wait_calls(MAX_READINESS_PROBES).await;
        let response = app.clone().oneshot(request("/ready")).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(database.calls.load(Ordering::SeqCst), MAX_READINESS_PROBES);
        assert_eq!(
            app.clone()
                .oneshot(request("/health"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let cancelled = tasks.pop().unwrap();
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());
        tasks.push(tokio::spawn(app.oneshot(request("/ready"))));
        wait_calls(MAX_READINESS_PROBES + 1).await;
        database.release.add_permits(MAX_READINESS_PROBES);
        for task in tasks {
            task.await.unwrap().unwrap();
        }
    }
}
