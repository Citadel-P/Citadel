use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, RawQuery, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_swarm_services::{
    CreateSwarmServiceInput, ManagedSwarmServiceService, RenameSwarmServiceInput,
    ResourceCapabilities, ScaleSwarmServiceInput, SwarmServiceChangeNotifier, SwarmServiceError,
    SwarmServiceFilter, UpdateSwarmServiceInput,
};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::realtime::RealtimeHub;

pub struct SwarmServicesRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}
impl SwarmServicesRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}
impl SwarmServiceChangeNotifier for SwarmServicesRealtimeNotifier {
    fn changed(&self, id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("SwarmService", id, event);
        }
    }
}

#[derive(Clone)]
pub struct SwarmServicesHttpState {
    pub identity: Arc<IdentityService>,
    pub services: Arc<ManagedSwarmServiceService>,
}

pub fn router(state: SwarmServicesHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_MANAGED_SWARM_SERVICES, list)
        .contract_route(routes::CREATE_SWARM_SERVICE, create)
        .contract_route(routes::DELETE_SWARM_SERVICES, delete)
        .contract_route(routes::GET_MANAGED_SWARM_SERVICE, get)
        .contract_route(routes::GET_SWARM_SERVICE_DUPLICATE_DRAFT, duplicate_draft)
        .contract_route(routes::GET_SWARM_SERVICE_ADOPTION_DRAFT, adoption_draft)
        .contract_route(routes::ADOPT_SWARM_SERVICE, adopt)
        .contract_route(routes::UPDATE_SWARM_SERVICE, update)
        .contract_route(routes::RENAME_SWARM_SERVICE, rename)
        .contract_route(routes::UPDATE_SWARM_SERVICE_METADATA, update_metadata)
        .contract_route(routes::APPLY_SWARM_SERVICE, apply)
        .contract_route(routes::SCALE_SWARM_SERVICE, scale)
        .contract_route(routes::FORCE_UPDATE_SWARM_SERVICE, force_update)
        .contract_route(routes::CHECK_SWARM_SERVICE_UPDATES, check_updates)
        .with_state(state)
}

async fn duplicate_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
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
            .services
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(serde_json::json!({
        "draft": {"name":draft.name,"platformId":draft.platform_id,"description":draft.description,
            "spec":draft.spec,"tagIds":draft.tag_ids,"duplicateSource":{
                "resourceType":"SwarmService","resourceId":id,"resourceName":draft.source_name}},
        "warnings":draft.warnings
    })).into_response()))
}

async fn authorize_adoption(
    state: &SwarmServicesHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::SwarmService,
                    Uuid::nil(),
                    PermissionLevel::Write,
                    None,
                )
                .await,
            headers,
        )?;
        identity_result(
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

async fn adoption_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let draft = identity_result(
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
    Ok(no_store(Json(serde_json::json!({
        "draft": {"name":draft.name,"platformId":draft.source.platform_id,"description":draft.description,
            "spec":draft.spec,"tagIds":null,"duplicateSource":null},
        "source":draft.source,"issues":draft.issues,"previewFingerprint":draft.preview_fingerprint
    })).into_response()))
}

async fn adopt(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<citadel_swarm_services::adoption::AdoptSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let service = identity_result(
        state
            .services
            .adopt(
                principal.actor_id,
                principal.is_administrator(),
                platform,
                &id,
                input,
                &cancel,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(service).into_response()))
}

#[derive(serde::Deserialize)]
struct ServiceMetadataInput {
    #[serde(deserialize_with = "deserialize_description")]
    description: Option<String>,
}

fn deserialize_description<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    serde::Deserialize::deserialize(deserializer)
}

async fn update_metadata(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ServiceMetadataInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
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
    Ok(no_store(Json(value).into_response()))
}

async fn check_updates(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = identity_result(
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
                SwarmServiceError::Runtime(message) => IdentityError::External(message),
                other => service_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn list(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = identity_result(parse_filter(query.as_deref()), &headers)?;
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let value = identity_result(
        state
            .services
            .list(
                principal.actor_id,
                principal.is_administrator(),
                &filter,
                capabilities,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}
async fn get(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .services
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}
async fn create(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::SwarmService,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .services
            .create(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}
async fn update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<UpdateSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .services
            .update(principal.actor_id, principal.is_administrator(), id, input)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}
async fn rename(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .services
            .rename(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}
async fn delete(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        state
            .services
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn apply(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    Ok(progress_response(state.services.apply(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}
async fn scale(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ScaleSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Write,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    Ok(progress_response(state.services.scale(
        principal.actor_id,
        principal.is_administrator(),
        id,
        input.replicas,
    )))
}
async fn force_update(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        id,
        PermissionLevel::Read,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    Ok(progress_response(state.services.force_update(
        principal.actor_id,
        principal.is_administrator(),
        id,
    )))
}
fn progress_response(
    mut receiver: tokio::sync::mpsc::Receiver<citadel_swarm_services::SwarmServiceProgressItem>,
) -> axum::response::Response {
    let stream = async_stream::stream! {yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));let mut first=true;while let Some(item)=receiver.recv().await{if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;match serde_json::to_vec(&item){Ok(value)=>yield Ok(bytes::Bytes::from(value)),Err(error)=>{tracing::error!(%error,"failed to serialize Swarm Service progress");break;}}}yield Ok(bytes::Bytes::from_static(b"]"));};
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
async fn authorize(
    state: &SwarmServicesHttpState,
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
        .authorize_resource(principal, ResourceType::SwarmService, id, level, specific)
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
        .global_permission(principal, ResourceType::SwarmService)
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
fn parse_filter(query: Option<&str>) -> Result<SwarmServiceFilter, IdentityError> {
    let mut filter = SwarmServiceFilter::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key.eq_ignore_ascii_case("tags") {
            if filter.tags.len() >= 100 {
                return Err(IdentityError::Validation(
                    "At most 100 Service Tags may be filtered at once.".to_owned(),
                ));
            }
            if !value.trim().is_empty() && !filter.tags.iter().any(|item| item == &value) {
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
pub(crate) fn service_error(error: SwarmServiceError) -> IdentityError {
    match error {
        SwarmServiceError::Validation(value) => crate::request_validation::validation_error(value),
        SwarmServiceError::NotFound => IdentityError::NotFound,
        SwarmServiceError::Forbidden => IdentityError::Forbidden,
        SwarmServiceError::Conflict(value)
        | SwarmServiceError::Runtime(value)
        | SwarmServiceError::RuntimeRejected(value) => IdentityError::Conflict(value),
        SwarmServiceError::Storage(value) => IdentityError::Storage(value),
        SwarmServiceError::Cancelled => {
            IdentityError::Conflict("The Service operation was cancelled.".to_owned())
        }
    }
}
fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(IdentityError::Unauthenticated)
}
fn invalid_path(_: PathRejection) -> IdentityError {
    IdentityError::Validation("The managed Swarm Service path is invalid.".to_owned())
}
fn invalid_json(error: JsonRejection) -> IdentityError {
    crate::request_validation::invalid_json(error)
}
