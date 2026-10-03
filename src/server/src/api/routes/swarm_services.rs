//! Swarm services HTTP routes, authorization and local request handling.
use crate::api::resource_access::collection_capabilities;
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::swarm_services::{
            requests::{ServiceMetadataInput, *},
            views::*,
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{WorkloadQuery, invalid_json, invalid_path},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{
        Extension, Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, PermissionPolicy, ResourceType, SpecificPermission};

use citadel_swarm_services::{
    SwarmServiceError, SwarmServiceFilter, SwarmServiceService, permissions as policy,
};

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct SwarmServicesHttpState {
    pub identity: Arc<IdentityService>,
    pub services: Arc<SwarmServiceService>,
}

pub fn router(state: SwarmServicesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<SwarmServicesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(delete))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(adoption_draft))
        .normalized_routes(utoipa_axum::routes!(adopt))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(update_metadata))
        .normalized_routes(utoipa_axum::routes!(apply))
        .normalized_routes(utoipa_axum::routes!(scale))
        .normalized_routes(utoipa_axum::routes!(force_update))
        .normalized_routes(utoipa_axum::routes!(check_updates))
}

fn progress_response(
    mut receiver: tokio::sync::mpsc::Receiver<citadel_swarm_services::SwarmServiceProgressItem>,
) -> axum::response::Response {
    let stream = async_stream::stream! {yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));let mut first=true;while let Some(item)=receiver.recv().await{if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;match serde_json::to_vec(&SwarmServiceProgressItem::from(item)){Ok(value)=>yield Ok(bytes::Bytes::from(value)),Err(error)=>{tracing::error!(%error,"failed to serialize Swarm Service progress");break;}}}yield Ok(bytes::Bytes::from_static(b"]"));};
    let mut response = Body::from_stream(stream).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn authorize<P: PermissionPolicy>(
    state: &SwarmServicesHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .require_resource::<P>(principal, id)
        .await
        .map_err(|error| crate::api::error::HttpError::from_parts(error, headers))
}

pub(crate) fn service_error(error: SwarmServiceError) -> ApiError {
    match error {
        SwarmServiceError::Validation(value) => crate::request_validation::validation_error(value),
        SwarmServiceError::NotFound => ApiError::NotFound,
        SwarmServiceError::Forbidden => ApiError::Forbidden,
        SwarmServiceError::Conflict(value)
        | SwarmServiceError::Runtime(value)
        | SwarmServiceError::RuntimeRejected(value) => ApiError::Conflict(value),
        source @ SwarmServiceError::Storage(_) => ApiError::internal(source),
        SwarmServiceError::Cancelled => {
            ApiError::Conflict("The Service operation was cancelled.".to_owned())
        }
    }
}

fn require_actor(principal: Option<Extension<ActorPrincipal>>) -> Result<ActorPrincipal, ApiError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(ApiError::Unauthenticated)
}

async fn authorize_adoption(
    state: &SwarmServicesHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_scope::<policy::CreateSwarmService>(principal)
                .await,
            headers,
        )?;
        api_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            headers,
        )?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adoption-draft",
    operation_id = "getSwarmServiceAdoptionDraft",
    tag = "Platforms",
    summary = "Review an unmanaged Docker Service for adoption",
    responses(
        (status = 200, description = "Success", body = SwarmServiceAdoptionDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn adoption_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let draft = api_result(
        state
            .services
            .adoption_draft(
                principal.actor_id,
                principal.is_administrator(),
                platform,
                &id,
                &cancel,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            SwarmServiceAdoptionDraftView::try_from(draft).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adopt",
    operation_id = "adoptSwarmService",
    tag = "Platforms",
    summary = "Adopt an existing Docker Service without changing Docker",
    request_body = AdoptSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn adopt(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<AdoptSwarmServiceInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let service = api_result(
        state
            .services
            .adopt(
                principal.actor_id,
                principal.is_administrator(),
                platform,
                &id,
                input.into(),
                &cancel,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(service).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/swarmServices/{id}/_metadata",
    operation_id = "updateSwarmServiceMetadata",
    tag = "SwarmServices",
    summary = "Update managed Service metadata",
    request_body(content(
        (ServiceMetadataInput = "application/merge-patch+json"),
        (ServiceMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_metadata(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ServiceMetadataInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let value = api_result(
        state
            .services
            .update_description(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.description.as_deref(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices",
    operation_id = "createSwarmService",
    tag = "SwarmServices",
    summary = "Create a managed Docker Swarm Service",
    request_body = CreateSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateSwarmServiceInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<policy::CreateSwarmService>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let value = api_result(
        state
            .services
            .create(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/swarmServices/{id}",
    operation_id = "updateSwarmService",
    tag = "SwarmServices",
    summary = "Update managed Docker Swarm Service configuration",
    request_body = UpdateSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<UpdateSwarmServiceInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let value = api_result(
        state
            .services
            .update(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/rename",
    operation_id = "renameSwarmService",
    tag = "SwarmServices",
    summary = "Rename a managed Docker Swarm Service",
    request_body = RenameSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameSwarmServiceInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, input.id, &headers).await?;
    let value = api_result(
        state
            .services
            .rename(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/swarmServices",
    operation_id = "deleteSwarmServices",
    tag = "SwarmServices",
    summary = "Delete managed Docker Swarm Services",
    request_body = Vec<Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(ids) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        state
            .services
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/check-updates",
    operation_id = "checkSwarmServiceUpdates",
    tag = "SwarmServices",
    summary = "Check the applied Service image for updates",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn check_updates(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::WriteSwarmService>(&state, &principal, id, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = api_result(
        state
            .services
            .check_updates(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            )
            .await
            .map_err(|error| match error {
                SwarmServiceError::Runtime(message) => ApiError::External(message),
                other => service_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/apply",
    operation_id = "applySwarmService",
    tag = "SwarmServices",
    summary = "Apply a managed Docker Swarm Service and stream progress",
    responses(
        (status = 200, description = "Success", body = Vec<SwarmServiceProgressItem>, content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn apply(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ApplySwarmService>(&state, &principal, id, &headers).await?;
    Ok(progress_response(state.services.apply(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/scale",
    operation_id = "scaleSwarmService",
    tag = "SwarmServices",
    summary = "Scale a managed Docker Swarm Service and stream progress",
    request_body = ScaleSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = Vec<SwarmServiceProgressItem>, content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn scale(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ScaleSwarmServiceInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ScaleSwarmService>(&state, &principal, id, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    Ok(progress_response(state.services.scale(
        principal.actor_id,
        principal.is_administrator(),
        id,
        input.replicas,
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/swarmServices/{id}/force-update",
    operation_id = "forceUpdateSwarmService",
    tag = "SwarmServices",
    summary = "Force a managed Docker Swarm Service task update and stream progress",
    responses(
        (status = 200, description = "Success", body = Vec<SwarmServiceProgressItem>, content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn force_update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ForceUpdateSwarmService>(&state, &principal, id, &headers).await?;
    Ok(progress_response(state.services.force_update(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/duplicate-draft",
    operation_id = "getSwarmServiceDuplicateDraft",
    tag = "SwarmServices",
    summary = "Prepare a managed Service duplicate",
    responses(
        (status = 200, description = "Success", body = SwarmServiceDuplicateDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn duplicate_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ReadSwarmService>(&state, &principal, id, &headers).await?;
    let draft = api_result(
        state
            .services
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            SwarmServiceDuplicateDraftView::from_draft(draft, id).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices",
    operation_id = "listManagedSwarmServices",
    tag = "SwarmServices",
    summary = "List managed Docker Swarm Services",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServicesView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::api::error::HttpError>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = SwarmServiceFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(
        &state.identity,
        &principal,
        ResourceType::SwarmService,
        &headers,
    )
    .await?;
    let value = api_result(
        state
            .services
            .list(principal.actor_id, principal.is_administrator(), &filter)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServicesView {
            swarm_services: value
                .into_iter()
                .map(ManagedSwarmServiceView::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    crate::api::error::HttpError::from_parts(ApiError::internal(error), &headers)
                })?,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}",
    operation_id = "getManagedSwarmService",
    tag = "SwarmServices",
    summary = "Get a managed Docker Swarm Service",
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize::<policy::ReadSwarmService>(&state, &principal, id, &headers).await?;
    let value = api_result(
        state
            .services
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ManagedSwarmServiceView::try_from(value).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}
