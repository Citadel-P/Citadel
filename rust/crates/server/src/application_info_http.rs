use axum::Json;
use axum::Router;
use axum::extract::Extension;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use citadel_identity::ActorPrincipal;
use citadel_identity::IdentityError;
use serde::Serialize;

use crate::identity_http::{identity_error_response, no_store};
use crate::openapi::router::OpenApiRouterExt;

const NAME: &str = "Citadel";
const VERSION: &str = env!("CITADEL_BUILD_VERSION");
const INFORMATIONAL_VERSION: &str = env!("CITADEL_BUILD_INFORMATIONAL_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfoView {
    pub name: &'static str,
    pub version: &'static str,
    pub informational_version: &'static str,
    pub realtime_transport: &'static str,
}

pub fn router() -> Router {
    documented_routes().split_for_parts().0
}

#[utoipa::path(
    get,
    path = "/api/v1/application/info",
    operation_id = "getApplicationInfo",
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
    headers: HeaderMap,
) -> Response {
    let Some(Extension(_principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    no_store(Json(application_info()).into_response())
}

#[must_use]
pub const fn application_info() -> ApplicationInfoView {
    ApplicationInfoView {
        name: NAME,
        version: VERSION,
        informational_version: INFORMATIONAL_VERSION,
        realtime_transport: "WebSocketV1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_version_omits_only_build_metadata() {
        let info = application_info();
        assert_eq!(info.name, "Citadel");
        assert!(!info.version.is_empty());
        assert!(!info.informational_version.is_empty());
        assert_eq!(info.realtime_transport, "WebSocketV1");
        assert_eq!(
            info.version,
            info.informational_version
                .split_once('+')
                .map_or(info.informational_version, |(display, _)| display)
        );
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<()> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_application_info))
}
