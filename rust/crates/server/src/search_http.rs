use crate::{
    identity_http::{IdentityHttpResult, identity_result, no_store},
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityError};

use citadel_discovery::GlobalSearchQuery;
use citadel_discovery::GlobalSearchReader;
use citadel_discovery::SearchError;

use std::sync::Arc;

pub fn router(store: Arc<dyn GlobalSearchReader>) -> Router {
    documented_routes().split_for_parts().0.with_state(store)
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    operation_id = "globalSearch",
    tag = "Search",
    summary = "Search authorized resources",
    responses(
        (status = 200, description = "Success", body = crate::api::discovery::dto::GlobalSearchResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("q" = String, Query), ("types" = Option<String>, Query), ("limitPerType" = Option<i32>, Query, minimum = 1, maximum = 10, extensions(("x-citadel-default" = json!(5))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn search(
    State(store): State<Arc<dyn GlobalSearchReader>>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<GlobalSearchQuery>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(
        principal
            .map(|Extension(actor)| actor)
            .ok_or(IdentityError::Unauthenticated),
        &headers,
    )?;
    let Query(query) = identity_result(
        query.map_err(|_| IdentityError::Validation("Invalid search query.".into())),
        &headers,
    )?;
    let query = identity_result(query.validate().map_err(error), &headers)?;
    let matches = identity_result(
        store
            .search(principal.actor_id, principal.is_administrator(), &query)
            .await
            .map_err(error),
        &headers,
    )?;
    let response = identity_result(
        citadel_discovery::GlobalSearchResults::from_matches(query.query, matches).map_err(error),
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::discovery::dto::GlobalSearchResponse::from(
            response,
        ))
        .into_response(),
    ))
}

fn error(error: SearchError) -> IdentityError {
    match error {
        SearchError::Validation(message) => IdentityError::Validation(message),
        other => IdentityError::Storage(other.to_string()),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<Arc<dyn GlobalSearchReader>>
{
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(search))
}
