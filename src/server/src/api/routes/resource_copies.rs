use crate::api::error::{ApiError, HttpResult, api_result, no_store};
use crate::api::request_validation::{ApiPath, ApiQuery};
use crate::api::resources::resource_copies::*;
use crate::openapi::router::OpenApiRouterExt;
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
};
use citadel_identity::ActorPrincipal;
use citadel_resource_copies::{CopyError, CopyRequest, CopyService};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ResourceCopiesHttpState {
    pub copies: Arc<CopyService>,
}
pub fn router(state: ResourceCopiesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}
pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ResourceCopiesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(preview_resource_copy))
        .normalized_routes(utoipa_axum::routes!(resource_copy_targets))
        .normalized_routes(utoipa_axum::routes!(start_resource_copy))
        .normalized_routes(utoipa_axum::routes!(get_resource_copy))
        .normalized_routes(utoipa_axum::routes!(cancel_resource_copy))
        .normalized_routes(utoipa_axum::routes!(cleanup_resource_copy))
}
fn actor(p: Option<Extension<ActorPrincipal>>, h: &HeaderMap) -> HttpResult<ActorPrincipal> {
    api_result(p.map(|Extension(p)| p).ok_or(ApiError::Unauthenticated), h)
}
fn result<T>(r: Result<T, CopyError>, h: &HeaderMap) -> HttpResult<T> {
    api_result(
        r.map_err(|e| match e {
            CopyError::Validation(m) => ApiError::Validation(m),
            CopyError::Conflict(m) => ApiError::Conflict(m),
            CopyError::NotFound => ApiError::NotFound,
            e @ CopyError::Storage(_) => ApiError::internal(e),
        }),
        h,
    )
}

#[utoipa::path(post,path="/api/v1/resource-copies/preview",tag="ResourceCopies",security(("Bearer" = [])),operation_id="previewResourceCopy",request_body=ResourceCopyRequest,responses((status=200,body=ResourceCopyPreview),crate::openapi::errors::ResourceMutationErrors))]
async fn preview_resource_copy(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    input: Result<Json<CopyRequest>, axum::extract::rejection::JsonRejection>,
) -> HttpResult {
    let p = actor(p, &h)?;
    let Json(input) = api_result(
        input.map_err(|_| ApiError::Validation("Invalid resource copy request.".into())),
        &h,
    )?;
    let preview = result(s.copies.preview(p.actor_id, input).await, &h)?;
    Ok(no_store(Json(preview).into_response()))
}
#[utoipa::path(post,path="/api/v1/resource-copies/{id}/start",tag="ResourceCopies",security(("Bearer" = [])),operation_id="startResourceCopy",params(("id"=Uuid,Path)),responses((status=200,body=ResourceCopyProgress),crate::openapi::errors::ResourceMutationErrors))]
async fn start_resource_copy(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(result(s.copies.start(p.actor_id, id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(get,path="/api/v1/resource-copies/{id}",tag="ResourceCopies",security(("Bearer" = [])),operation_id="getResourceCopy",params(("id"=Uuid,Path)),responses((status=200,body=ResourceCopyProgress),crate::openapi::errors::ResourceMutationErrors))]
async fn get_resource_copy(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(result(s.copies.get(p.actor_id, id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(post,path="/api/v1/resource-copies/{id}/cancel",tag="ResourceCopies",security(("Bearer" = [])),operation_id="cancelResourceCopy",params(("id"=Uuid,Path)),responses((status=200,body=ResourceCopyProgress),crate::openapi::errors::ResourceMutationErrors))]
async fn cancel_resource_copy(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(result(s.copies.cancel(p.actor_id, id).await, &h)?).into_response(),
    ))
}

#[utoipa::path(post,path="/api/v1/resource-copies/{id}/cleanup",tag="ResourceCopies",security(("Bearer" = [])),operation_id="cleanupResourceCopy",params(("id"=Uuid,Path)),responses((status=200,body=ResourceCopyProgress),crate::openapi::errors::ResourceMutationErrors))]
async fn cleanup_resource_copy(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(result(s.copies.cleanup(p.actor_id, id).await, &h)?).into_response(),
    ))
}

#[utoipa::path(get,path="/api/v1/resource-copies/targets",tag="ResourceCopies",security(("Bearer" = [])),operation_id="getResourceCopyTargets",params(("kind"=ResourceCopyKind,Query),("sourceId"=Uuid,Query)),responses((status=200,body=ResourceCopyTargets),crate::openapi::errors::ResourceMutationErrors))]
async fn resource_copy_targets(
    State(s): State<ResourceCopiesHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ApiQuery(query): ApiQuery<ResourceCopyTargetsQuery>,
) -> HttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(result(
            s.copies
                .targets(p.actor_id, query.kind, query.source_id)
                .await,
            &h,
        )?)
        .into_response(),
    ))
}
