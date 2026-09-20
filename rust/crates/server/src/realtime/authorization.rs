use super::{
    PLATFORM_RESOURCE_TYPE, REALTIME_PROTOCOL_VERSION, RealtimeError, RealtimeService,
    protocol::ClientMessage,
};
use crate::api::resources::platforms::views::{ContainerView, PlatformView};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_platforms::PlatformReadService;
use citadel_primitives::{PermissionLevel, ResourceType};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use uuid::Uuid;
use zeroize::Zeroizing;

pub trait RealtimeReadPort: Send + Sync {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>>;

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>>;

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, RealtimeReadError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum RealtimeReadError {
    #[error("authentication failed")]
    Authentication,
    #[error("authorization failed")]
    Authorization,
    #[error("realtime read failed: {0}")]
    Storage(String),
}

pub struct IdentityRealtimeReader {
    identity: Arc<IdentityService>,
    platforms: Arc<PlatformReadService>,
}

impl IdentityRealtimeReader {
    #[must_use]
    pub fn new(identity: Arc<IdentityService>, platforms: Arc<PlatformReadService>) -> Self {
        Self {
            identity,
            platforms,
        }
    }
}

impl RealtimeReadPort for IdentityRealtimeReader {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>> {
        Box::pin(async move {
            crate::token_safety::authenticate_realtime(&self.identity, token)
                .await
                .map_err(|error| match error {
                    citadel_identity::IdentityError::Forbidden => RealtimeReadError::Authorization,
                    _ => RealtimeReadError::Authentication,
                })
        })
    }

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        Box::pin(async move {
            if !principal.is_administrator() {
                let permission = self
                    .identity
                    .permission_for_resource(principal, ResourceType::Platform, platform_id)
                    .await
                    .map_err(|error| RealtimeReadError::Storage(error.to_string()))?;
                if !permission
                    .is_some_and(|permission| permission.level.grants(PermissionLevel::Read))
                {
                    return Err(RealtimeReadError::Authorization);
                }
            }
            self.platforms
                .get_platform(platform_id)
                .await
                .map(|value| value.map(crate::api::resources::platforms::views::PlatformView::from))
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
                .ok_or(RealtimeReadError::Authorization)
        })
    }

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, RealtimeReadError>> {
        Box::pin(async move {
            self.platforms
                .list_containers(platform_id)
                .await
                .map(|value| {
                    value
                        .into_iter()
                        .map(crate::api::resources::platforms::views::ContainerView::from)
                        .collect::<Vec<_>>()
                })
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))
        })
    }
}

pub(super) struct Subscription {
    pub(super) principal: ActorPrincipal,
    pub(super) platform_id: Option<Uuid>,
    pub(super) access_token: Zeroizing<String>,
}

pub(super) async fn validate_subscription(
    service: &RealtimeService,
    subscribe: ClientMessage,
) -> Result<Subscription, RealtimeError> {
    if subscribe.protocol_version != REALTIME_PROTOCOL_VERSION {
        return Err(RealtimeError::UnsupportedProtocol(
            subscribe.protocol_version,
        ));
    }
    if subscribe.kind != "subscribe" {
        return Err(RealtimeError::InvalidMessage(
            "a realtime subscription is required".to_owned(),
        ));
    }
    let Some(access_token) = subscribe.access_token.map(Zeroizing::new) else {
        service.inner.metrics.realtime_authorization_failed();
        return Err(RealtimeError::Authentication);
    };
    let principal = service
        .inner
        .reader
        .authenticate(access_token.as_str())
        .await
        .map_err(map_realtime_read_error)?;
    let platform_id = match (subscribe.resource_type.as_deref(), subscribe.resource_id) {
        (None, None) => None,
        (Some(PLATFORM_RESOURCE_TYPE), Some(platform_id)) => {
            authorize_platform(service, &principal, platform_id).await?;
            Some(platform_id)
        }
        _ => {
            return Err(RealtimeError::InvalidMessage(
                "resourceType and resourceId must identify a Platform, or both be omitted"
                    .to_owned(),
            ));
        }
    };
    Ok(Subscription {
        principal,
        platform_id,
        access_token,
    })
}

pub(super) async fn authorize_platform(
    service: &RealtimeService,
    principal: &ActorPrincipal,
    platform_id: Uuid,
) -> Result<PlatformView, RealtimeError> {
    let result = service
        .inner
        .reader
        .authorize_platform(principal, platform_id)
        .await
        .map_err(map_realtime_read_error);
    if result.is_err() {
        service.inner.metrics.realtime_authorization_failed();
    }
    result
}

pub(super) fn map_realtime_read_error(error: RealtimeReadError) -> RealtimeError {
    match error {
        RealtimeReadError::Authentication => RealtimeError::Authentication,
        RealtimeReadError::Authorization => RealtimeError::Authorization,
        RealtimeReadError::Storage(message) => RealtimeError::AuthorizationStorage(message),
    }
}
