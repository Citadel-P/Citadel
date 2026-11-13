//! Reads shared for one published event only, including transient process snapshots.
//! Callers authorize
//! before reading and construct actor-specific views after reading.
use super::retention::{Budget, Cache};
use super::{PublishedRuntimeEvent, RealtimeReadError};
use citadel_platforms::{ContainerDetails, ImageDetails, PlatformDetails, PlatformReadService};
use citadel_runtime::runtime_metrics::RuntimeWork;
use uuid::Uuid;

#[derive(Debug, Default)]
pub(crate) struct CommittedReads {
    pub(crate) budget: Budget,
    pub(crate) _payload: Option<tokio::sync::OwnedSemaphorePermit>,
    pub(crate) processes: tokio::sync::Mutex<
        std::collections::BTreeMap<Uuid, std::sync::Arc<Cache<serde_json::Value>>>,
    >,

    identities: Cache<Vec<citadel_platforms::ContainerIdentity>>,
    telemetry: Cache<Option<citadel_platforms::PlatformTelemetryContext>>,
    platform: Cache<Option<PlatformDetails>>,
    swarm_summary: Cache<citadel_platforms::swarm_overview::SwarmSummary>,
    containers: Cache<Vec<ContainerDetails>>,
    images: Cache<Vec<ImageDetails>>,
    pub(crate) networks: Cache<Vec<citadel_platforms::RuntimeNetworkSummary>>,
    pub(crate) volumes: Cache<Vec<citadel_platforms::RuntimeVolumeSummary>>,
    node_networks: Cache<
        Vec<citadel_platforms::NodeResourceProjection<citadel_platforms::RuntimeNetworkSummary>>,
    >,
    node_volumes: Cache<
        Vec<citadel_platforms::NodeResourceProjection<citadel_platforms::RuntimeVolumeSummary>>,
    >,
}

macro_rules! shared_read {
    ($name:ident, $field:ident, $method:ident, $result:ty) => {
        pub(crate) async fn $name(
            service: &PlatformReadService,
            platform: Uuid,
            event: Option<&PublishedRuntimeEvent>,
        ) -> Result<$result, RealtimeReadError> {
            let read = || async {
                let _read = RuntimeWork::RealtimeSharedRead.start();
                service
                    .$method(platform)
                    .await
                    .map_err(|e| RealtimeReadError::Storage(e.to_string()))
            };
            match event.filter(|event| event.platform_id == Some(platform)) {
                Some(event) => Ok(event.reads.$field.get_or_try_init(read).await?.clone()),
                None => read().await,
            }
        }
    };
}
pub(crate) async fn platform<'a>(
    service: &PlatformReadService,
    platform: Uuid,
    event: Option<&'a PublishedRuntimeEvent>,
) -> Result<std::borrow::Cow<'a, Option<PlatformDetails>>, RealtimeReadError> {
    let read = || async {
        let _read = RuntimeWork::RealtimeSharedRead.start();
        service
            .get_platform(platform)
            .await
            .map_err(|e| RealtimeReadError::Storage(e.to_string()))
    };
    match event.filter(|e| e.platform_id == Some(platform)) {
        Some(event) => Ok(std::borrow::Cow::Borrowed(
            event.reads.platform.get_or_try_init(read).await?,
        )),
        None => Ok(std::borrow::Cow::Owned(read().await?)),
    }
}
shared_read!(
    swarm_summary,
    swarm_summary,
    swarm_summary,
    citadel_platforms::swarm_overview::SwarmSummary
);
shared_read!(
    containers,
    containers,
    list_containers,
    Vec<ContainerDetails>
);
shared_read!(images, images, list_images, Vec<ImageDetails>);

shared_read!(
    node_networks,
    node_networks,
    list_node_networks,
    Vec<citadel_platforms::NodeResourceProjection<citadel_platforms::RuntimeNetworkSummary>>
);
shared_read!(
    node_volumes,
    node_volumes,
    list_node_volumes,
    Vec<citadel_platforms::NodeResourceProjection<citadel_platforms::RuntimeVolumeSummary>>
);

pub(crate) async fn identities<'a>(
    service: &PlatformReadService,
    platform: Uuid,
    event: Option<&'a PublishedRuntimeEvent>,
) -> Result<std::borrow::Cow<'a, [citadel_platforms::ContainerIdentity]>, RealtimeReadError> {
    let read = || async {
        let _read = RuntimeWork::RealtimeSharedRead.start();
        service
            .container_identities(platform)
            .await
            .map_err(|e| RealtimeReadError::Storage(e.to_string()))
    };
    match event.filter(|e| e.platform_id == Some(platform)) {
        Some(event) => Ok(std::borrow::Cow::Borrowed(
            event.reads.identities.get_or_try_init(read).await?,
        )),
        None => Ok(std::borrow::Cow::Owned(read().await?)),
    }
}
pub(crate) async fn telemetry<'a>(
    service: &PlatformReadService,
    platform: Uuid,
    event: Option<&'a PublishedRuntimeEvent>,
) -> Result<
    std::borrow::Cow<'a, Option<citadel_platforms::PlatformTelemetryContext>>,
    RealtimeReadError,
> {
    let read = || async {
        let _read = RuntimeWork::RealtimeSharedRead.start();
        service
            .platform_telemetry(platform)
            .await
            .map_err(|e| RealtimeReadError::Storage(e.to_string()))
    };
    match event.filter(|e| e.platform_id == Some(platform)) {
        Some(event) => Ok(std::borrow::Cow::Borrowed(
            event.reads.telemetry.get_or_try_init(read).await?,
        )),
        None => Ok(std::borrow::Cow::Owned(read().await?)),
    }
}

impl CommittedReads {
    pub(crate) fn new(budget: Budget, payload: tokio::sync::OwnedSemaphorePermit) -> Self {
        Self {
            _payload: Some(payload),
            processes: Default::default(),
            identities: Cache::new(budget.clone()),
            telemetry: Cache::new(budget.clone()),
            platform: Cache::new(budget.clone()),
            swarm_summary: Cache::new(budget.clone()),
            containers: Cache::new(budget.clone()),
            images: Cache::new(budget.clone()),
            networks: Cache::new(budget.clone()),
            volumes: Cache::new(budget.clone()),
            node_networks: Cache::new(budget.clone()),
            node_volumes: Cache::new(budget.clone()),
            budget,
        }
    }
}
