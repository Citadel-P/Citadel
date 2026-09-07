use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, RawQuery, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_deployments::{
    ApplyDeploymentInput, CreateDeploymentInput, DeploymentChangeNotifier, DeploymentError,
    DeploymentFilter, DeploymentService, PatchDeploymentInput, PatchDeploymentMetadataInput,
    RenameDeploymentInput, ResourceCapabilities,
};
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::realtime::RealtimeHub;

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
    Router::new()
        .contract_route(routes::LIST_DEPLOYMENTS, list_deployments)
        .contract_route(routes::CREATE_DEPLOYMENT, create_deployment)
        .contract_route(routes::APPLY_DEPLOYMENT, apply_deployment)
        .contract_route(routes::DELETE_DEPLOYMENTS, delete_deployments)
        .contract_route(routes::RENAME_DEPLOYMENT, rename_deployment)
        .contract_route(routes::GET_DEPLOYMENT, get_deployment)
        .contract_route(routes::CHECK_DEPLOYMENT_UPDATES, check_deployment_updates)
        .contract_route(routes::GET_DEPLOYMENT_CONFIG, get_deployment_config)
        .contract_route(
            routes::GET_DEPLOYMENT_DUPLICATE_DRAFT,
            get_deployment_duplicate_draft,
        )
        .contract_route(routes::UPDATE_DEPLOYMENT, update_deployment)
        .contract_route(
            routes::UPDATE_DEPLOYMENT_METADATA,
            update_deployment_metadata,
        )
        .with_state(state)
}

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

async fn list_deployments(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = identity_result(parse_filter(raw_query.as_deref()), &headers)?;
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

fn parse_filter(query: Option<&str>) -> Result<DeploymentFilter, IdentityError> {
    let mut filter = DeploymentFilter::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key.eq_ignore_ascii_case("tags") {
            if filter.tags.len() >= 100 {
                return Err(IdentityError::Validation(
                    "At most 100 Deployment Tags may be filtered at once.".to_owned(),
                ));
            }
            if !value.trim().is_empty() && !filter.tags.iter().any(|tag| tag == &value) {
                filter.tags.push(value.into_owned());
            }
        } else if key.eq_ignore_ascii_case("platformId") {
            let id = Uuid::parse_str(&value).map_err(|_| {
                IdentityError::Validation("The Platform ID filter is invalid.".to_owned())
            })?;
            if filter.platform_id.replace(id).is_some() {
                return Err(IdentityError::Validation(
                    "The Platform ID filter may be supplied only once.".to_owned(),
                ));
            }
        }
    }
    Ok(filter)
}

fn deployment_error(error: DeploymentError) -> IdentityError {
    match error {
        DeploymentError::Validation(message) => IdentityError::Validation(message),
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

fn invalid_path(_: PathRejection) -> IdentityError {
    IdentityError::Validation("The Deployment path is invalid.".to_owned())
}

fn invalid_json(_: JsonRejection) -> IdentityError {
    IdentityError::Validation("The request body is invalid.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_accepts_repeated_tags_and_one_platform() {
        let platform = Uuid::now_v7();
        let parsed =
            parse_filter(Some(&format!("tags=prod&tags=blue&platformId={platform}"))).unwrap();
        assert_eq!(parsed.tags, ["prod", "blue"]);
        assert_eq!(parsed.platform_id, Some(platform));
    }

    #[test]
    fn filter_rejects_repeated_platform() {
        let platform = Uuid::now_v7();
        assert!(
            parse_filter(Some(&format!(
                "platformId={platform}&platformId={platform}"
            )))
            .is_err()
        );
    }
}
