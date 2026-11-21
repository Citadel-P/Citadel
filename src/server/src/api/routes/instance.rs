use crate::api::resources::instance::requests::JobHistoryQuery;
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        request_validation::ValidatedJson,
    },
    openapi::router::OpenApiRouterExt,
};
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use citadel_identity::{ActorPrincipal, IdentityError};
use citadel_instance::{InstanceError, InstanceService};
use std::sync::Arc;

#[derive(Clone)]
pub struct InstanceHttpState {
    pub instance: Arc<InstanceService>,
}

pub fn router(state: InstanceHttpState) -> Router {
    crate::api::mutation_notifications::notify_mutations(
        settings_routes()
            .split_for_parts()
            .0
            .with_state(state.clone()),
        "InstallationSettings",
    )
    // Restart admission does not change settings. Broadcasting their invalidation
    // here would trigger reads while Core shuts down; reconnect reconciles them.
    .merge(restart_routes().split_for_parts().0.with_state(state))
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<InstanceHttpState> {
    settings_routes().merge(restart_routes())
}

fn settings_routes() -> utoipa_axum::router::OpenApiRouter<InstanceHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_installation_settings))
        .normalized_routes(utoipa_axum::routes!(get_database_connections))
        .normalized_routes(utoipa_axum::routes!(get_schedules))
        .normalized_routes(utoipa_axum::routes!(get_jobs))
        .normalized_routes(utoipa_axum::routes!(get_job_history))
        .normalized_routes(utoipa_axum::routes!(patch_installation_settings))
}

fn restart_routes() -> utoipa_axum::router::OpenApiRouter<InstanceHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_instance_restart))
        .normalized_routes(utoipa_axum::routes!(restart_instance))
}

impl From<InstanceError> for ApiError {
    fn from(error: InstanceError) -> Self {
        match error {
            InstanceError::Forbidden => ApiError::Forbidden,
            InstanceError::Validation(message) => ApiError::Validation(message),
            InstanceError::Conflict(message) => ApiError::Conflict(message),
            source @ InstanceError::Storage(_) => ApiError::internal(source),
        }
    }
}

#[utoipa::path(get, path = "/api/v1/instance/database-connections", operation_id = "getDatabaseConnections", tag = "Instance",
    summary = "Get this Core's database connection snapshot (administrator)",
    responses((status = 200, description = "Query pool usage and connected PostgreSQL shared buffers", body = crate::api::resources::instance::views::DatabaseConnectionsView), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn get_database_connections(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.instance.database_connections(&principal).await {
        Ok(snapshot) => no_store(
            Json(crate::api::resources::instance::views::DatabaseConnectionsView::from(snapshot))
                .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/installation/settings",
    operation_id = "getInstallationSettings",
    tag = "Setup",
    summary = "Get installation settings (administrator)",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::instance::views::InstallationSettingsView),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_installation_settings(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.instance.get_installation_settings(&principal).await {
        Ok(settings) => no_store(
            Json(
                crate::api::resources::instance::views::InstallationSettingsView::new(
                    settings,
                    state.instance.installation_policy(),
                ),
            )
            .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/installation/settings",
    operation_id = "patchInstallationSettings",
    tag = "Setup",
    summary = "Update installation settings (administrator)",
    request_body = crate::api::resources::instance::requests::PatchInstallationSettingsRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::instance::views::InstallationSettingsView),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch_installation_settings(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<
        crate::api::resources::instance::requests::PatchInstallationSettingsRequest,
    >,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .instance
        .patch_installation_settings(&principal, request.into())
        .await
    {
        Ok(settings) => no_store(
            Json(
                crate::api::resources::instance::views::InstallationSettingsView::new(
                    settings,
                    state.instance.installation_policy(),
                ),
            )
            .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(get, path = "/api/v1/instance/restart", operation_id = "getInstanceRestart", tag = "Instance",
    summary = "Get Core restart availability (administrator)",
    responses((status = 200, description = "Core restart availability and current boot identity", body = crate::api::resources::instance::views::InstanceRestartView), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn get_instance_restart(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    if !principal.is_administrator() {
        return error_response(InstanceError::Forbidden, &headers);
    }
    let view = match state.instance.restart() {
        Some(service) => match service.status(&principal).await {
            Ok(status) => crate::api::resources::instance::views::InstanceRestartView {
                available: status.available,
                pending: status.pending,
                boot_id: Some(status.boot_id),
            },
            Err(error) => return error_response(error, &headers),
        },
        None => crate::api::resources::instance::views::InstanceRestartView {
            available: false,
            pending: false,
            boot_id: None,
        },
    };
    no_store(Json(view).into_response())
}

#[utoipa::path(post, path = "/api/v1/instance/restart", operation_id = "restartInstance", tag = "Instance",
    summary = "Restart Core to apply saved configuration (administrator)",
    request_body = crate::api::resources::instance::requests::RestartInstanceRequest,
    responses((status = 202, description = "Restart accepted"), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn restart_instance(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<
        crate::api::resources::instance::requests::RestartInstanceRequest,
    >,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    if !principal.is_administrator() {
        return error_response(InstanceError::Forbidden, &headers);
    }
    let Some(service) = state.instance.restart() else {
        return error_response(
            InstanceError::Conflict("In-app Core restart is unavailable on this host.".into()),
            &headers,
        );
    };
    match service.request(&principal, request.boot_id).await {
        Ok(()) => no_store(axum::http::StatusCode::ACCEPTED.into_response()),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(get, path = "/api/v1/instance/jobs", operation_id = "getInstanceJobs", tag = "Instance",
    summary = "Get this Core's background jobs (administrator)",
    responses((status = 200, description = "Process-local job observations", body = crate::api::resources::instance::jobs::JobsSnapshotView), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn get_jobs(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.instance.jobs(&principal) {
        Ok(value) => no_store(
            Json(crate::api::resources::instance::jobs::JobsSnapshotView::from(value))
                .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(get, path = "/api/v1/instance/jobs/history", operation_id = "getInstanceJobHistory", tag = "Instance",
    params(JobHistoryQuery),
    summary = "Get retained job diagnostics (administrator)",
    responses((status = 200, description = "Bounded history across Core sessions", body = crate::api::resources::instance::job_history::JobHistoryView), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn get_job_history(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    axum::extract::Query(query): axum::extract::Query<JobHistoryQuery>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .instance
        .job_history(&principal, query.job.as_deref(), query.hours)
        .await
    {
        Ok(history) => no_store(
            Json(crate::api::resources::instance::job_history::JobHistoryView::from(history))
                .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

use crate::api::resources::instance::schedules::{SchedulePage, ScheduleQuery};
#[utoipa::path(get, path = "/api/v1/instance/schedules", operation_id = "getInstanceSchedules", tag = "Instance",
    params(ScheduleQuery), summary = "List resource schedules (administrator)",
    responses((status = 200, description = "One page of schedule definitions", body = SchedulePage), crate::openapi::errors::AccessErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false))))]
async fn get_schedules(
    State(state): State<InstanceHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    axum::extract::Query(query): axum::extract::Query<ScheduleQuery>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    let result = async {
        state
            .instance
            .schedules(&principal, query.purpose()?, query.cursor()?)
            .await
    }
    .await;
    match result {
        Ok(rows) => no_store(Json(SchedulePage::from(rows)).into_response()),
        Err(e) => error_response(e, &headers),
    }
}
