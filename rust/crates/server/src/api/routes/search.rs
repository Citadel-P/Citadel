use crate::{
    api::error::{ApiError, HttpResult, api_result, no_store},
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};

use citadel_discovery::{GlobalSearchQuery, GlobalSearchReader, SearchError};

use citadel_identity::ActorPrincipal;

use std::sync::Arc;

pub fn router(store: Arc<dyn GlobalSearchReader>) -> Router {
    documented_routes().split_for_parts().0.with_state(store)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<Arc<dyn GlobalSearchReader>>
{
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(search))
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    operation_id = "globalSearch",
    tag = "Search",
    summary = "Search authorized resources",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::search::views::GlobalSearchResponse, content_type = "application/json"),
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
) -> HttpResult {
    let principal = api_result(
        principal
            .map(|Extension(actor)| actor)
            .ok_or(ApiError::Unauthenticated),
        &headers,
    )?;
    let Query(query) = api_result(
        query.map_err(|_| ApiError::Validation("Invalid search query.".into())),
        &headers,
    )?;
    let query = api_result(query.validate().map_err(error), &headers)?;
    let matches = api_result(
        store
            .search(principal.actor_id, principal.is_administrator(), &query)
            .await
            .map_err(error),
        &headers,
    )?;
    let response = api_result(
        citadel_discovery::GlobalSearchResults::from_matches(query.query, matches).map_err(error),
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::search::views::GlobalSearchResponse::from(response))
            .into_response(),
    ))
}

fn error(error: SearchError) -> ApiError {
    match error {
        SearchError::Validation(message) => ApiError::Validation(message),
        other => ApiError::internal(other),
    }
}
