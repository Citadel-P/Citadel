use crate::{
    identity_http::{IdentityHttpResult, identity_result, no_store, require_human_administrator},
    openapi::router::OpenApiRouterExt,
};

use axum::extract::rejection::{JsonRejection, PathRejection};

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::HeaderMap,
    response::IntoResponse,
};

use citadel_identity::IdentityError;

use citadel_identity::{ActorPrincipal, ActorRepository};

use crate::identity_http::dto::PatchActorEnabledInput;

use std::sync::Arc;

use uuid::Uuid;

pub fn router(store: Arc<dyn ActorRepository>) -> Router {
    documented_routes().split_for_parts().0.with_state(store)
}

#[utoipa::path(
    get,
    path = "/api/v1/actors/{id}",
    operation_id = "getActor",
    tag = "Actors",
    summary = "Get an Actor",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::ActorView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(store): State<Arc<dyn ActorRepository>>,
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

#[utoipa::path(
    patch,
    path = "/api/v1/actors/{id}/enabled",
    operation_id = "patchActorEnabled",
    tag = "Actors",
    summary = "Enable or disable an Actor",
    request_body = PatchActorEnabledInput,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::ActorView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn set_enabled(
    State(store): State<Arc<dyn ActorRepository>>,
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
    let input: citadel_identity::PatchActorEnabledInput = input.into();
    Ok(no_store(
        Json(identity_result(
            store.set_enabled(id, input.is_enabled).await,
            &headers,
        )?)
        .into_response(),
    ))
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<Arc<dyn ActorRepository>> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(set_enabled))
}
