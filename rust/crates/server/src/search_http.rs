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
use citadel_resources::{
    GlobalSearchQuery, GlobalSearchResponse, GlobalSearchStore, ResourceMetadataError,
};
use std::sync::Arc;

pub fn router(store: Arc<dyn GlobalSearchStore>) -> Router {
    documented_routes().split_for_parts().0.with_state(store)
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    operation_id = "globalSearch",
    tag = "Search",
    summary = "Search authorized resources",
    responses(
        (status = 200, description = "Success", body = citadel_resources::GlobalSearchResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("q" = String, Query), ("types" = Option<String>, Query), ("limitPerType" = Option<i32>, Query, minimum = 1, maximum = 10, extensions(("x-citadel-default" = json!(5))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn search(
    State(store): State<Arc<dyn GlobalSearchStore>>,
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
        GlobalSearchResponse::from_matches(query.query, matches).map_err(error),
        &headers,
    )?;
    Ok(no_store(Json(response).into_response()))
}

fn error(error: ResourceMetadataError) -> IdentityError {
    match error {
        ResourceMetadataError::Validation(message) => IdentityError::Validation(message),
        other => IdentityError::Storage(other.to_string()),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<Arc<dyn GlobalSearchStore>>
{
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(search))
}
