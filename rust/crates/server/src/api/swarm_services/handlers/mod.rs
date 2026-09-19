use super::{requests::*, spec::*, views::*};

use citadel_swarm_services::permissions as policy;

use citadel_domain::PermissionPolicy;

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

use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use citadel_swarm_services::{
    SwarmServiceChangeNotifier, SwarmServiceError, SwarmServiceFilter, SwarmServiceService,
};

use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

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
    pub services: Arc<SwarmServiceService>,
}

pub fn router(state: SwarmServicesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
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
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .require_resource::<P>(principal, id)
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<SwarmServicesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(read::list))
        .normalized_routes(utoipa_axum::routes!(mutations::create))
        .normalized_routes(utoipa_axum::routes!(mutations::delete))
        .normalized_routes(utoipa_axum::routes!(read::get))
        .normalized_routes(utoipa_axum::routes!(read::duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(adoption::adoption_draft))
        .normalized_routes(utoipa_axum::routes!(adoption::adopt))
        .normalized_routes(utoipa_axum::routes!(mutations::update))
        .normalized_routes(utoipa_axum::routes!(mutations::rename))
        .normalized_routes(utoipa_axum::routes!(mutations::update_metadata))
        .normalized_routes(utoipa_axum::routes!(operations::apply))
        .normalized_routes(utoipa_axum::routes!(operations::scale))
        .normalized_routes(utoipa_axum::routes!(operations::force_update))
        .normalized_routes(utoipa_axum::routes!(operations::check_updates))
}
mod adoption;

mod operations;

mod read;

mod mutations;
