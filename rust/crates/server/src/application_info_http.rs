use axum::Json;
use axum::Router;
use axum::extract::Extension;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_application::IdentityError;
use citadel_domain::ActorPrincipal;
use serde::Serialize;

use crate::identity_http::{identity_error_response, no_store};

const NAME: &str = "Citadel";
const VERSION: &str = env!("CITADEL_BUILD_VERSION");
const INFORMATIONAL_VERSION: &str = env!("CITADEL_BUILD_INFORMATIONAL_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfoView {
    pub name: &'static str,
    pub version: &'static str,
    pub informational_version: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/api/v1/application/info", get(get_application_info))
}

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
        assert_eq!(
            info.version,
            info.informational_version
                .split_once('+')
                .map_or(info.informational_version, |(display, _)| display)
        );
    }
}
