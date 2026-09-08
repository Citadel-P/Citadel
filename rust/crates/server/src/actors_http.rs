use crate::{
    contract_router::ContractRouterExt,
    identity_http::{IdentityHttpResult, identity_result, no_store, require_human_administrator},
};
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use citadel_contracts::http::routes;
use citadel_identity::IdentityError;
use citadel_identity::{ActorPrincipal, ActorStore, PatchActorEnabledInput};
use std::sync::Arc;
use uuid::Uuid;

pub fn router(store: Arc<dyn ActorStore>) -> Router {
    Router::new()
        .contract_route(routes::GET_ACTOR, get)
        .contract_route(routes::PATCH_ACTOR_ENABLED, set_enabled)
        .with_state(store)
}

async fn get(
    State(store): State<Arc<dyn ActorStore>>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    identity_result(require_human_administrator(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Invalid Actor id.".into())),
        &headers,
    )?;
    Ok(no_store(
        Json(identity_result(store.get(id).await, &headers)?).into_response(),
    ))
}

async fn set_enabled(
    State(store): State<Arc<dyn ActorStore>>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchActorEnabledInput>, JsonRejection>,
) -> IdentityHttpResult {
    identity_result(require_human_administrator(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|_| IdentityError::Validation("Invalid Actor id.".into())),
        &headers,
    )?;
    let Json(input) = identity_result(
        input.map_err(|_| IdentityError::Validation("The request body is invalid.".into())),
        &headers,
    )?;
    Ok(no_store(
        Json(identity_result(
            store.set_enabled(id, input.is_enabled).await,
            &headers,
        )?)
        .into_response(),
    ))
}
