use crate::{Readiness, metrics::Metrics, openapi::router::OpenApiRouterExt};
use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct DiagnosticsHttpState {
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub pool: PgPool,
}

#[derive(Serialize, utoipa::ToSchema)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: DiagnosticsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
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
async fn ready(State(state): State<DiagnosticsHttpState>) -> Response {
    let readiness = state.readiness.snapshot();
    let status = if readiness.database && readiness.docker {
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
        state.metrics.render(&state.pool),
    )
        .into_response()
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DiagnosticsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(health))
        .normalized_routes(utoipa_axum::routes!(ready))
        .normalized_routes(utoipa_axum::routes!(open_metrics))
}
