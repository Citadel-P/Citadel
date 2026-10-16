//! Additional image operations use the existing Platform authorization and runtime owner.
use super::platforms::{
    PlatformsHttpState, authorize_platform_level, platform_is_swarm, require_actor,
};
use crate::api::{
    error::{ApiError, HttpResult, api_result, no_store, runtime_error_response},
    request_validation::{invalid_json, invalid_path, invalid_query},
    resources::{platforms::image_management::*, schema_models::image_management::*},
};
use crate::openapi::router::OpenApiRouterExt;
use axum::{
    Json,
    extract::{
        Extension, Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use citadel_identity::ActorPrincipal;
use citadel_platforms::image_usage::{ImageLocation, preview_deletion};
use citadel_primitives::PermissionLevel;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<PlatformsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(check_image_update))
        .normalized_routes(utoipa_axum::routes!(image_usage))
        .normalized_routes(utoipa_axum::routes!(preview_image_deletion))
}

#[utoipa::path(get,path="/api/v1/images/{platformId}/{imageId}/usage",operation_id="getImageUsage",tag="Images",
    params(("platformId"=Uuid,Path),("imageId"=String,Path),("dockerNodeId"=Option<String>,Query),("offset"=Option<u32>,Query)),
    responses((status=200,body=ImageUsageSchema),crate::openapi::errors::ExternalResourceErrors),security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("actor")),("x-citadel-public"=json!(true)),("x-citadel-setup-exempt"=json!(false))))]
async fn image_usage(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ImageUsageQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    let ids = api_result(normalize_ids(vec![id]), &headers)?;
    if query.offset > 100_000 {
        return api_result(
            Err(ApiError::Validation("Usage offset is too large.".into())),
            &headers,
        );
    }
    authorize_platform_level(
        &state,
        &principal,
        platform,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let result = state
        .image_store
        .usage(
            principal.actor_id,
            principal.is_administrator(),
            platform,
            &ImageLocation {
                docker_image_id: ids[0].clone(),
                docker_node_id: query.docker_node_id,
            },
            query.offset,
        )
        .await;
    match result {
        Ok(usage) => Ok(no_store(
            Json(ImageUsageSchema::from(usage)).into_response(),
        )),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(post,path="/api/v1/images/deletion-preview",operation_id="previewImageDeletion",tag="Images",
    request_body=PreviewImageDeletionInput,
    responses((status=200,body=Vec<ImageDeletionPreviewItemSchema>),crate::openapi::errors::ExternalResourceErrors),security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("actor")),("x-citadel-public"=json!(true)),("x-citadel-setup-exempt"=json!(false))))]
async fn preview_image_deletion(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<PreviewImageDeletionInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let ids = api_result(normalize_ids(input.ids), &headers)?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return api_result(
            Err(ApiError::Conflict(
                "Node-local Image deletion is not available.".into(),
            )),
            &headers,
        );
    }
    static PREVIEW_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let _permit = api_result(
        PREVIEW_SLOTS.try_acquire().map_err(|_| {
            ApiError::Conflict("Image review capacity is busy. Try again later.".into())
        }),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        preview_deletion(
            state.image_store.as_ref(),
            state.runtime.as_ref(),
            principal.actor_id,
            principal.is_administrator(),
            input.platform_id,
            &ids,
            &cancel,
        ),
    )
    .await;
    match result {
        Ok(Ok(items)) => Ok(no_store(
            Json(
                items
                    .into_iter()
                    .map(ImageDeletionPreviewItemSchema::from)
                    .collect::<Vec<_>>(),
            )
            .into_response(),
        )),
        Ok(Err(error)) => Ok(runtime_error_response(error, &headers)),
        Err(_) => api_result(
            Err(ApiError::Conflict(
                "Live image review timed out. Refresh inventory before retrying.".into(),
            )),
            &headers,
        ),
    }
}

pub(super) fn normalize_ids(mut ids: Vec<String>) -> Result<Vec<String>, ApiError> {
    if ids.is_empty() || ids.len() > 100 {
        return Err(ApiError::Validation(
            "Select between 1 and 100 images.".into(),
        ));
    }
    for id in &mut ids {
        let hash = id.strip_prefix("sha256:").unwrap_or(id);
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ApiError::Validation(
                "Full Docker image IDs are required.".into(),
            ));
        }
        *id = format!("sha256:{}", hash.to_ascii_lowercase());
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[utoipa::path(post,path="/api/v1/images/check-update",operation_id="checkImageUpdate",tag="Images",
    request_body=CheckImageUpdateInput,
    responses((status=200,body=ImageTagUpdateSchema),crate::openapi::errors::ExternalResourceErrors),security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("actor")),("x-citadel-public"=json!(true)),("x-citadel-setup-exempt"=json!(false))))]
async fn check_image_update(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CheckImageUpdateInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let ids = api_result(normalize_ids(vec![input.docker_image_id]), &headers)?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    if input.reference.len() > 2048 || input.reference.chars().any(char::is_whitespace) {
        return api_result(
            Err(ApiError::Validation("Select a valid image tag.".into())),
            &headers,
        );
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    match state
        .image_updates
        .check(
            principal.actor_id,
            principal.is_administrator(),
            input.platform_id,
            &ImageLocation {
                docker_image_id: ids[0].clone(),
                docker_node_id: input.docker_node_id,
            },
            input.registry_id,
            &input.reference,
            &cancel,
        )
        .await
    {
        Ok(result) => Ok(no_store(
            Json(ImageTagUpdateSchema::from(result)).into_response(),
        )),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}
