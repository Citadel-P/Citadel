//! Deployments HTTP routes, authorization and local request handling.
use crate::api::resource_access::collection_capabilities;
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        request_validation::{WorkloadQuery, invalid_json, invalid_path},
        resources::deployments::{
            adoption_views::ContainerAdoptionDraft,
            requests::{AdoptContainerInput, *},
            views::{DeploymentView, *},
        },
    },
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    extract::{
        Extension, Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
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
    pub copy_eligibility: Option<Arc<citadel_resource_copies::CopyService>>,
    pub backup_coverage:
        Option<Arc<crate::api::resources::backups::coverage::BackupCoverageService>>,
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
    let receiver = api_result(
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
    Ok(progress_response(receiver))
}

fn progress_response(
    receiver: tokio::sync::mpsc::Receiver<citadel_deployments::DeploymentProgress>,
) -> axum::response::Response {
    use futures_util::StreamExt;
    crate::api::progress::json_response(
        tokio_stream::wrappers::ReceiverStream::new(receiver).map(DeploymentStreamItem::from),
    )
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
    let mut views = api_result(result, &headers)?
        .into_iter()
        .map(DeploymentView::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            crate::api::error::HttpError::from_parts(ApiError::internal(error), &headers)
        })?;
    if let Some(service) = &state.backup_coverage {
        views = service.enrich_deployments(&principal, views).await;
    }
    if let Some(service) = &state.copy_eligibility {
        crate::api::resources::resource_copies::enrich_deployments(
            service,
            principal.actor_id,
            &mut views,
        )
        .await;
    }
    Ok(no_store(
        Json(DeploymentsView {
            deployments: views,
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
    let mut view = api_result(
        DeploymentView::try_from(deployment).map_err(ApiError::internal),
        &headers,
    )?;
    if let Some(service) = &state.backup_coverage {
        let platform = view.platform_id;
        view.backup_coverage = Some(
            service
                .workload(
                    &principal,
                    citadel_backups::policies::read_models::BackupPreviewKind::Deployment,
                    id,
                    platform,
                )
                .await,
        );
    }
    if let Some(service) = &state.copy_eligibility {
        view.copy_eligibility = Some(
            crate::api::resources::resource_copies::eligibility(
                service,
                principal.actor_id,
                citadel_resource_copies::CopyKind::Deployment,
                id,
            )
            .await,
        );
    }
    Ok(no_store(Json(view).into_response()))
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
            crate::api::request_validation::validation_error(message)
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
    use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
    use citadel_deployments::DeploymentProgress;
    use futures_util::StreamExt;

    #[tokio::test(start_paused = true)]
    async fn apply_progress_keeps_idle_connections_alive_without_changing_items() {
        for terminal in [
            DeploymentProgress::info("Deployment completed."),
            DeploymentProgress::failure(1, "Image pull failed."),
        ] {
            let (sender, receiver) = tokio::sync::mpsc::channel(4);
            let response = progress_response(receiver);
            assert_eq!(response.headers()["x-accel-buffering"], "no");
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
            assert_eq!(
                response.headers()[CONTENT_TYPE],
                "application/json; charset=utf-8"
            );
            let mut stream = response.into_body().into_data_stream();
            let mut body = stream.next().await.unwrap().unwrap().to_vec();
            assert_eq!(body, b"[");

            // Keepalive also works before the first progress item.
            let started = tokio::time::Instant::now();
            body.extend_from_slice(&stream.next().await.unwrap().unwrap());
            assert_eq!(started.elapsed(), std::time::Duration::from_secs(15));
            let pulling = DeploymentProgress::info("Pulling image codeberg.org/forgejo/forgejo:15");
            sender.send(pulling.clone()).await.unwrap();
            body.extend_from_slice(&stream.next().await.unwrap().unwrap());

            // A pull can stay silent longer than a two-minute proxy idle limit.
            for _ in 0..9 {
                let started = tokio::time::Instant::now();
                let chunk = stream.next().await.unwrap().unwrap();
                assert_eq!(chunk.as_ref(), b"\n");
                assert_eq!(started.elapsed(), std::time::Duration::from_secs(15));
                body.extend_from_slice(&chunk);
            }
            sender.send(terminal.clone()).await.unwrap();
            drop(sender);
            while let Some(chunk) = stream.next().await {
                body.extend_from_slice(&chunk.unwrap());
            }
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                serde_json::json!([
                    DeploymentStreamItem::from(pulling),
                    DeploymentStreamItem::from(terminal),
                ]),
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn apply_progress_closes_when_producer_finishes_without_waiting_for_keepalive() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        drop(sender);
        let started = tokio::time::Instant::now();
        let body = axum::body::to_bytes(progress_response(receiver).into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), b"[]");
        assert_eq!(started.elapsed(), std::time::Duration::ZERO);
    }

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
