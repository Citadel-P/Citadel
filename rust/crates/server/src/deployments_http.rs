use crate::request_validation::WorkloadQuery;
use crate::request_validation::{invalid_json, invalid_path};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_deployments::{
    ApplyDeploymentInput, CreateDeploymentInput, DeploymentChangeNotifier, DeploymentError,
    DeploymentFilter, DeploymentService, PatchDeploymentInput, PatchDeploymentMetadataInput,
    RenameDeploymentInput, ResourceCapabilities,
};
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;
use crate::realtime::RealtimeHub;

#[path = "deployments_http/adoption.rs"]
mod adoption;

pub struct DeploymentsRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}

impl DeploymentsRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}

impl DeploymentChangeNotifier for DeploymentsRealtimeNotifier {
    fn adopted(&self, deployment: &citadel_deployments::DeploymentView) {
        self.changed(deployment.id, "created");
        if let Some(realtime) = &self.realtime {
            if let Some(container) = &deployment.docker_container_id {
                realtime.publish_runtime_change(
                    deployment.platform_id,
                    "container",
                    "update",
                    container,
                );
            }
            realtime.publish_resource_change("Platform", deployment.platform_id, "updated");
        }
    }
    fn changed(&self, deployment_id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("Deployment", deployment_id, event);
        }
    }
}

#[derive(Clone)]
pub struct DeploymentsHttpState {
    pub identity: Arc<IdentityService>,
    pub deployments: Arc<DeploymentService>,
}

pub fn router(state: DeploymentsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/{deploymentId}/check-updates",
    operation_id = "checkDeploymentUpdates",
    summary = "Check the applied Deployment image for updates",
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let cancellation = tokio_util::sync::CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let checked = identity_result(
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
                DeploymentError::Runtime(message) => IdentityError::External(message),
                other => deployment_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(Json(checked).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/apply",
    operation_id = "applyDeployment",
    summary = "Apply a Deployment",
    request_body = ApplyDeploymentInput,
    responses(
        (status = 200, description = "Success", body = Vec<citadel_deployments::DeploymentStreamItem>, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_resource(
        &state,
        &principal,
        input.id,
        PermissionLevel::Execute,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    let mut receiver = identity_result(
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
            match serde_json::to_vec(&item) {
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
    summary = "List authorized Deployments",
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentsView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_deployments(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::identity_http::IdentityHttpError>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = DeploymentFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let result = state
        .deployments
        .list(
            principal.actor_id,
            principal.is_administrator(),
            &filter,
            capabilities,
        )
        .await
        .map_err(deployment_error);
    Ok(no_store(
        Json(identity_result(result, &headers)?).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}",
    operation_id = "getDeployment",
    summary = "Get a Deployment",
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let deployment = identity_result(
        state
            .deployments
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(Json(deployment).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/_cfg",
    operation_id = "getDeploymentConfig",
    summary = "Get Deployment configuration",
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentConfigView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let config = identity_result(
        state
            .deployments
            .get_config(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(Json(config).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/duplicate-draft",
    operation_id = "getDeploymentDuplicateDraft",
    summary = "Build a Deployment duplicate draft",
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentDuplicateDraftView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let draft = identity_result(
        state
            .deployments
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(Json(draft).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments",
    operation_id = "createDeployment",
    summary = "Create a Deployment",
    request_body = CreateDeploymentInput,
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Deployment,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let deployment = identity_result(
        state
            .deployments
            .create(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(Json(deployment).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/deployments/{id}",
    operation_id = "updateDeployment",
    summary = "Update Deployment configuration",
    request_body = PatchDeploymentInput,
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let deployment = identity_result(
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
    Ok(no_store(Json(deployment).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/deployments/{id}/_metadata",
    operation_id = "updateDeploymentMetadata",
    summary = "Update Deployment metadata",
    request_body = PatchDeploymentMetadataInput,
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_resource(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let deployment = identity_result(
        state
            .deployments
            .update_metadata(principal.actor_id, principal.is_administrator(), id, input)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(Json(deployment).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/deployments/rename",
    operation_id = "renameDeployment",
    summary = "Rename a Deployment",
    request_body = RenameDeploymentInput,
    responses(
        (status = 200, description = "Success", body = citadel_deployments::DeploymentView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_resource(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let deployment = identity_result(
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
    Ok(no_store(Json(deployment).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/deployments",
    operation_id = "deleteDeployments",
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        state
            .deployments
            .delete(principal.actor_id, principal.is_administrator(), ids)
            .await
            .map_err(deployment_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn authorize_resource(
    state: &DeploymentsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .authorize_resource(principal, ResourceType::Deployment, id, level, specific)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))
}

async fn collection_capabilities(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilities> {
    if principal.is_administrator() {
        return Ok(ResourceCapabilities {
            can_read: true,
            can_write: true,
            can_execute: true,
        });
    }
    let permission = identity
        .global_permission(principal, ResourceType::Deployment)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))?;
    Ok(ResourceCapabilities {
        can_read: permission
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Read)),
        can_write: permission
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Write)),
        can_execute: permission
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Execute)),
    })
}

fn deployment_error(error: DeploymentError) -> IdentityError {
    match error {
        DeploymentError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        DeploymentError::NotFound => IdentityError::NotFound,
        DeploymentError::Forbidden => IdentityError::Forbidden,
        DeploymentError::LicenseRequired(capability) => IdentityError::LicenseRequired(capability),
        DeploymentError::Conflict(message) | DeploymentError::Runtime(message) => {
            IdentityError::Conflict(message)
        }
        DeploymentError::Storage(message) => IdentityError::Storage(message),
        DeploymentError::Cancelled => {
            IdentityError::Conflict("The Deployment operation was cancelled.".to_owned())
        }
    }
}

fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(principal)| principal)
        .ok_or(IdentityError::Unauthenticated)
}

#[cfg(test)]
mod tests {
    use super::*;

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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DeploymentsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_deployments))
        .normalized_routes(utoipa_axum::routes!(create_deployment))
        .normalized_routes(utoipa_axum::routes!(adoption::draft))
        .normalized_routes(utoipa_axum::routes!(adoption::adopt))
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
