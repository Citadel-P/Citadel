//! Deployments HTTP routes, authorization and local request handling.
use crate::api::resource_access::collection_capabilities;
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::deployments::{
            adoption_views::ContainerAdoptionDraft,
            requests::{AdoptContainerInput, *},
            views::{DeploymentView, *},
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

use citadel_deployments::{
    DeploymentError, DeploymentFilter, DeploymentService,
    permissions::{ApplyDeployment, CreateDeployment, ReadDeployment, WriteDeployment},
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::ResourceType;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

#[derive(Clone)]
pub struct DeploymentsHttpState {
    pub identity: Arc<IdentityService>,
    pub deployments: Arc<DeploymentService>,
}

pub fn router(state: DeploymentsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DeploymentsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_deployments))
        .normalized_routes(utoipa_axum::routes!(create_deployment))
        .normalized_routes(utoipa_axum::routes!(crate::api::routes::deployments::draft))
        .normalized_routes(utoipa_axum::routes!(crate::api::routes::deployments::adopt))
        .normalized_routes(utoipa_axum::routes!(apply_deployment))
        .normalized_routes(utoipa_axum::routes!(delete_deployments))
        .normalized_routes(utoipa_axum::routes!(rename_deployment))
        .normalized_routes(utoipa_axum::routes!(get_deployment))
        .normalized_routes(utoipa_axum::routes!(check_deployment_updates))
        .normalized_routes(utoipa_axum::routes!(get_deployment_config))
        .normalized_routes(utoipa_axum::routes!(get_deployment_duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(update_deployment))
        .normalized_routes(utoipa_axum::routes!(update_deployment_metadata))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/{deploymentId}/check-updates",
    operation_id = "checkDeploymentUpdates",
    tag = "Deployments",
    summary = "Check the applied Deployment image for updates",
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn check_deployment_updates(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<WriteDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let cancellation = tokio_util::sync::CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let checked = api_result(
        state
            .deployments
            .check_updates(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancellation,
            )
            .await
            .map_err(|error| match error {
                DeploymentError::Runtime(message) => ApiError::External(message),
                other => deployment_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(checked).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/apply",
    operation_id = "applyDeployment",
    tag = "Deployments",
    summary = "Apply a Deployment",
    request_body = ApplyDeploymentInput,
    responses(
        (status = 200, description = "Success", body = Vec<DeploymentStreamItem>, content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn apply_deployment(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ApplyDeploymentInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<ApplyDeployment>(&principal, input.id)
            .await,
        &headers,
    )?;
    let mut receiver = api_result(
        state
            .deployments
            .apply(
                principal.actor_id,
                principal.is_administrator(),
                input.id,
                input.recreate.unwrap_or(false),
            )
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    let stream = async_stream::stream! {
        yield Ok::<_, std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first = true;
        while let Some(item) = receiver.recv().await {
            if !first {
                yield Ok(bytes::Bytes::from_static(b","));
            }
            first = false;
            match serde_json::to_vec(&DeploymentStreamItem::from(item)) {
                Ok(value) => yield Ok(bytes::Bytes::from(value)),
                Err(error) => {
                    tracing::error!(%error, "failed to serialize Deployment Apply progress");
                    break;
                }
            }
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = Body::from_stream(stream).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments",
    operation_id = "listDeployments",
    tag = "Deployments",
    summary = "List authorized Deployments",
    responses(
        (status = 200, description = "Success", body = DeploymentsView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_deployments(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::api::error::HttpError>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = DeploymentFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(
        &state.identity,
        &principal,
        ResourceType::Deployment,
        &headers,
    )
    .await?;
    let result = state
        .deployments
        .list(principal.actor_id, principal.is_administrator(), &filter)
        .await
        .map_err(deployment_error);
    Ok(no_store(
        Json(DeploymentsView {
            deployments: api_result(result, &headers)?
                .into_iter()
                .map(DeploymentView::try_from)
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
    path = "/api/v1/deployments/{deploymentId}",
    operation_id = "getDeployment",
    tag = "Deployments",
    summary = "Get a Deployment",
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_deployment(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<ReadDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let deployment = api_result(
        state
            .deployments
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/_cfg",
    operation_id = "getDeploymentConfig",
    tag = "Deployments",
    summary = "Get Deployment configuration",
    responses(
        (status = 200, description = "Success", body = DeploymentConfigView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_deployment_config(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<ReadDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let config = api_result(
        state
            .deployments
            .get_config(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(DeploymentConfigView::from(config)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/duplicate-draft",
    operation_id = "getDeploymentDuplicateDraft",
    tag = "Deployments",
    summary = "Build a Deployment duplicate draft",
    responses(
        (status = 200, description = "Success", body = DeploymentDuplicateDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_deployment_duplicate_draft(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<ReadDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let draft = api_result(
        state
            .deployments
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentDuplicateDraftView::try_from(draft).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments",
    operation_id = "createDeployment",
    tag = "Deployments",
    summary = "Create a Deployment",
    request_body = CreateDeploymentInput,
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_deployment(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateDeploymentInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<CreateDeployment>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let deployment = api_result(
        state
            .deployments
            .create(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/deployments/{id}",
    operation_id = "updateDeployment",
    tag = "Deployments",
    summary = "Update Deployment configuration",
    request_body(content(
        (PatchDeploymentInput = "application/merge-patch+json"),
        (PatchDeploymentInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_deployment(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchDeploymentInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<WriteDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let deployment = api_result(
        state
            .deployments
            .update_config(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.platform_id,
                input.spec,
            )
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/deployments/{id}/_metadata",
    operation_id = "updateDeploymentMetadata",
    tag = "Deployments",
    summary = "Update Deployment metadata",
    request_body(content(
        (PatchDeploymentMetadataInput = "application/merge-patch+json"),
        (PatchDeploymentMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_deployment_metadata(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchDeploymentMetadataInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<WriteDeployment>(&principal, id)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let deployment = api_result(
        state
            .deployments
            .update_metadata(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/rename",
    operation_id = "renameDeployment",
    tag = "Deployments",
    summary = "Rename a Deployment",
    request_body = RenameDeploymentInput,
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_deployment(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameDeploymentInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        state
            .identity
            .require_resource::<WriteDeployment>(&principal, input.id)
            .await,
        &headers,
    )?;
    let deployment = api_result(
        state
            .deployments
            .rename(
                principal.actor_id,
                principal.is_administrator(),
                input.id,
                input.name,
            )
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/deployments",
    operation_id = "deleteDeployments",
    tag = "Deployments",
    summary = "Delete Deployments",
    request_body = Vec<Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_deployments(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(ids) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        state
            .deployments
            .delete(principal.actor_id, principal.is_administrator(), ids)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

fn deployment_error(error: DeploymentError) -> ApiError {
    match error {
        DeploymentError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        DeploymentError::NotFound => ApiError::NotFound,
        DeploymentError::Forbidden => ApiError::Forbidden,
        DeploymentError::LicenseRequired(capability) => ApiError::LicenseRequired(capability),
        DeploymentError::Conflict(message) | DeploymentError::Runtime(message) => {
            ApiError::Conflict(message)
        }
        source @ DeploymentError::Storage(_) => ApiError::internal(source),
        DeploymentError::Cancelled => {
            ApiError::Conflict("The Deployment operation was cancelled.".to_owned())
        }
    }
}

fn require_actor(principal: Option<Extension<ActorPrincipal>>) -> Result<ActorPrincipal, ApiError> {
    principal
        .map(|Extension(principal)| principal)
        .ok_or(ApiError::Unauthenticated)
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/adoption-draft",
    operation_id = "getContainerAdoptionDraft",
    tag = "Containers",
    summary = "Review adoption of an unmanaged Container",
    responses(
        (status = 200, description = "Success", body = ContainerAdoptionDraft, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn draft(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<CreateDeployment>(&principal)
            .await,
        &headers,
    )?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let draft = api_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            state.deployments.adoption_draft(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            ),
        )
        .await
        .unwrap_or_else(|_| {
            Err(DeploymentError::Runtime(
                "Container adoption inspection timed out.".into(),
            ))
        })
        .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ContainerAdoptionDraft::try_from(draft).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/containers/{id}/adopt",
    operation_id = "adoptContainer",
    tag = "Containers",
    summary = "Adopt a Container without changing Docker",
    request_body = AdoptContainerInput,
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn adopt(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<AdoptContainerInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<CreateDeployment>(&principal)
            .await,
        &headers,
    )?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let deployment = api_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            state.deployments.adopt_container(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
                &cancel,
            ),
        )
        .await
        .unwrap_or_else(|_| {
            Err(DeploymentError::Runtime(
                "Container adoption timed out. Refresh inventory before retrying.".into(),
            ))
        })
        .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            DeploymentView::try_from(deployment).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::api::routes::deployments::*;

    #[test]
    fn filter_accepts_repeated_tags_and_one_platform() {
        let platform = Uuid::now_v7();
        let parsed =
            WorkloadQuery::parse(Some(&format!("tags=prod&tags=blue&platformId={platform}")))
                .unwrap();
        assert_eq!(parsed.tags, ["prod", "blue"]);
        assert_eq!(parsed.platform_id, Some(platform));
    }

    #[test]
    fn filter_rejects_repeated_platform() {
        let platform = Uuid::now_v7();
        assert!(
            WorkloadQuery::parse(Some(&format!(
                "platformId={platform}&platformId={platform}"
            )))
            .is_err()
        );
    }
}
