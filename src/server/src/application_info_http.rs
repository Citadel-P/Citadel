use crate::{
    api::error::{error_response, no_store},
    openapi::router::OpenApiRouterExt,
};
use axum::{
    Json, Router,
    extract::Extension,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use citadel_identity::{ActorPrincipal, IdentityError};
use serde::Serialize;

const NAME: &str = "Citadel";

const VERSION: &str = env!("CITADEL_BUILD_VERSION");

const INFORMATIONAL_VERSION: &str = env!("CITADEL_BUILD_INFORMATIONAL_VERSION");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfoView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_update: Option<crate::updates::AvailableUpdate>,
    pub name: &'static str,
    pub version: &'static str,
    pub informational_version: &'static str,
    pub realtime_transport: &'static str,
}

pub fn router(updates: crate::updates::UpdateChecker) -> Router {
    documented_routes()
        .split_for_parts()
        .0
        .layer(Extension(updates))
}

#[utoipa::path(
    get,
    path = "/api/v1/application/info",
    operation_id = "getApplicationInfo",
    tag = "Application",
    summary = "Get application information",
    responses(
        (status = 200, description = "Success", body = crate::application_info_http::ApplicationInfoView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_application_info(
    principal: Option<Extension<ActorPrincipal>>,
    Extension(updates): Extension<crate::updates::UpdateChecker>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(IdentityError::Unauthenticated, &headers);
    };
    let mut info = application_info();
    if principal.is_administrator() {
        info.available_update = updates.available().await;
    }
    no_store(Json(info).into_response())
}

#[must_use]
pub const fn application_info() -> ApplicationInfoView {
    ApplicationInfoView {
        available_update: None,
        name: NAME,
        version: VERSION,
        informational_version: INFORMATIONAL_VERSION,
        realtime_transport: "WebSocketV1",
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<()> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_application_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_info_exposes_each_embedded_version_independently() {
        let info = application_info();
        assert_eq!(info.name, "Citadel");
        assert!(!info.version.is_empty());
        assert!(!info.informational_version.is_empty());
        assert_eq!(info.realtime_transport, "WebSocketV1");
        assert_eq!(info.version, env!("CITADEL_BUILD_VERSION"));
        assert_eq!(
            info.informational_version,
            env!("CITADEL_BUILD_INFORMATIONAL_VERSION")
        );
    }
}
