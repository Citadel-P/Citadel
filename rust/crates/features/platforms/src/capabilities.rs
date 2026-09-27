//! Resource capabilities implemented directly by transport adapters.
use crate::*;
use futures_util::future::BoxFuture;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub trait PlatformInfoPort: Send + Sync {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>>;
}

/// A container-only dependency cannot call `get_info`.
/// ```compile_fail
/// use citadel_platforms::ContainerInventoryPort;
/// use tokio_util::sync::CancellationToken;
/// async fn unrelated(source: &dyn ContainerInventoryPort, cancel: &CancellationToken) {
///     source.get_info(cancel).await;
/// }
/// ```
/// A container-only dependency cannot call `list_images`.
/// ```compile_fail
/// use citadel_platforms::ContainerInventoryPort;
/// use tokio_util::sync::CancellationToken;
/// async fn unrelated(source: &dyn ContainerInventoryPort, cancel: &CancellationToken) {
///     source.list_images(cancel).await;
/// }
/// ```
/// A container-only dependency cannot call `list_networks`.
/// ```compile_fail
/// use citadel_platforms::ContainerInventoryPort;
/// use tokio_util::sync::CancellationToken;
/// async fn unrelated(source: &dyn ContainerInventoryPort, cancel: &CancellationToken) {
///     source.list_networks(cancel).await;
/// }
/// ```
/// A container-only dependency cannot call `list_volumes`.
/// ```compile_fail
/// use citadel_platforms::ContainerInventoryPort;
/// use tokio_util::sync::CancellationToken;
/// async fn unrelated(source: &dyn ContainerInventoryPort, cancel: &CancellationToken) {
///     source.list_volumes(cancel).await;
/// }
/// ```
pub trait ContainerInventoryPort: Send + Sync {
    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>>;
}

pub trait PlatformStatsPort: Send + Sync {
    fn stream_stats<'a>(
        &'a self,
        fetch_interval: Duration,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>>;
}

pub trait ImageInventoryPort: Send + Sync {
    fn list_images<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>>;
}

pub trait NetworkInventoryPort: Send + Sync {
    fn list_networks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>>;
}

pub trait NetworkObservationPort: Send + Sync {
    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>>;
}

pub trait VolumeInventoryPort: Send + Sync {
    fn list_volumes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>>;
}

pub trait VolumeObservationPort: Send + Sync {
    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>>;
}

pub trait SwarmInventoryPort: Send + Sync {
    fn list_swarm_nodes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmNode>, RuntimeCapabilityError>>;

    fn list_swarm_services<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmService>, RuntimeCapabilityError>>;

    fn list_swarm_tasks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>>;

    fn list_swarm_configs<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>>;

    fn list_swarm_secrets<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>>;
}

pub trait NetworkMutationPort: Send + Sync {
    fn create_network<'a>(
        &'a self,
        input: &'a CreateRuntimeNetwork,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>>;

    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}

pub trait VolumeMutationPort: Send + Sync {
    fn create_volume<'a>(
        &'a self,
        input: &'a CreateRuntimeVolume,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>>;

    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
}

/// Sampling supplied IDs requires neither discovery nor platform metadata.
/// ```compile_fail
/// use citadel_platforms::ContainerStatsPort;
/// use tokio_util::sync::CancellationToken;
/// async fn enumerate(source: &dyn ContainerStatsPort, cancel: &CancellationToken) {
///     source.list_containers(cancel).await;
/// }
/// ```
pub trait ContainerStatsPort: Send + Sync {
    fn sample_container_stats<'a>(
        &'a self,
        container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>>;
}
