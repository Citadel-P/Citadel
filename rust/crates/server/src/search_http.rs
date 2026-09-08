use crate::{
    contract_router::ContractRouterExt,
    identity_http::{IdentityHttpResult, identity_result, no_store},
};
use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use citadel_contracts::http::routes;
use citadel_identity::{ActorPrincipal, IdentityError};
use citadel_resources::{
    GlobalSearchQuery, GlobalSearchResponse, GlobalSearchStore, ResourceMetadataError,
};
use std::sync::Arc;

pub fn router(store: Arc<dyn GlobalSearchStore>) -> Router {
    Router::new()
        .contract_route(routes::GLOBAL_SEARCH, search)
        .with_state(store)
}

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
