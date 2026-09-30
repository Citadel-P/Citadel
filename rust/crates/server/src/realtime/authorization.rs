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
    fn authorization_changes(
        &self,
        _actor: citadel_primitives::ActorId,
    ) -> Option<tokio::sync::watch::Receiver<Uuid>> {
        None
    }

    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>>;

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>>;

    fn platform_for_event<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
        _event: &'a super::PublishedRuntimeEvent,
        _lease: &'a super::AuthorizationLease,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        self.authorize_platform(principal, platform_id)
    }

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
    fn authorization_changes(
        &self,
        actor: citadel_primitives::ActorId,
    ) -> Option<tokio::sync::watch::Receiver<Uuid>> {
        self.identity.authorization_changes(actor)
    }

    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>> {
        Box::pin(async move {
            let _authentication =
                citadel_runtime::runtime_metrics::RuntimeWork::RealtimeAuthentication.start();
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
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
                .map(PlatformView::try_from)
                .transpose()
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
                .ok_or(RealtimeReadError::Authorization)
        })
    }

    fn platform_for_event<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
        event: &'a super::PublishedRuntimeEvent,
        lease: &'a super::AuthorizationLease,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        Box::pin(async move {
            lease
                .authorize(
                    &self.identity,
                    principal,
                    ResourceType::Platform,
                    Some(platform_id),
                    PermissionLevel::Read,
                    None,
                )
                .await?;
            super::shared_reads::platform(&self.platforms, platform_id, Some(event))
                .await?
                .map(PlatformView::try_from)
                .transpose()
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
    pub(super) lease: super::AuthorizationLease,
    pub(super) authorization_changes: Option<tokio::sync::watch::Receiver<Uuid>>,
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
    let authorization_changes = service
        .inner
        .reader
        .authorization_changes(principal.actor_id);
    // Bind the watch before the authoritative authentication/authorization read.
    // A concurrent commit is retained by watch even before the connection loop.
    let principal = if authorization_changes.is_some() {
        let checked = service
            .inner
            .reader
            .authenticate(access_token.as_str())
            .await
            .map_err(map_realtime_read_error)?;
        if checked.actor_id != principal.actor_id {
            return Err(RealtimeError::Authentication);
        }
        checked
    } else {
        principal
    };
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
    if authorization_changes
        .as_ref()
        .is_some_and(|changes| changes.has_changed().unwrap_or(true))
    {
        return Err(RealtimeError::Authorization);
    }
    Ok(Subscription {
        lease: super::AuthorizationLease::new(principal.actor_id, authorization_changes.clone()),
        authorization_changes,
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

impl Subscription {
    pub(super) async fn recheck(&mut self, service: &RealtimeService) -> Result<(), RealtimeError> {
        let principal = service
            .inner
            .reader
            .authenticate(&self.access_token)
            .await
            .map_err(map_realtime_read_error)?;
        if principal.actor_id != self.principal.actor_id && self.lease.tracked() {
            return Err(RealtimeError::Authentication);
        }
        self.lease.clear();
        self.principal = principal;
        Ok(())
    }
}
