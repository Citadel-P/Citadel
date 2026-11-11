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
    responses((status = 200, body = crate::api::resources::instance::views::InstanceRestartView), crate::openapi::errors::AccessErrors),
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
