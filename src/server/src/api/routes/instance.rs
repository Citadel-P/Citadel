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
        documented_routes().split_for_parts().0.with_state(state),
        "InstallationSettings",
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<InstanceHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_installation_settings))
        .normalized_routes(utoipa_axum::routes!(patch_installation_settings))
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
