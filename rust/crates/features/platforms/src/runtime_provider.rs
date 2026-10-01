//! Resolution returns only the capabilities required by each consumer.
//! This boundary does not execute operations or own a second connection cache.
use crate::*;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
pub trait SwarmRuntimePort:
    swarm_mutations::SwarmControlPort + PlatformInfoPort + NetworkInventoryPort + SwarmInventoryPort
{
}
impl<
    T: swarm_mutations::SwarmControlPort
        + PlatformInfoPort
        + NetworkInventoryPort
        + SwarmInventoryPort
        + ?Sized,
> SwarmRuntimePort for T
{
}
pub trait ImageMutationRuntimePort:
    image_pull::ImagePullPort + images::ImageDeletionPort + ImageInventoryPort
{
}
impl<T: image_pull::ImagePullPort + images::ImageDeletionPort + ImageInventoryPort + ?Sized>
    ImageMutationRuntimePort for T
{
}
pub trait NetworkRuntimePort:
    NetworkInventoryPort + NetworkObservationPort + NetworkMutationPort
{
}
impl<T: NetworkInventoryPort + NetworkObservationPort + NetworkMutationPort + ?Sized>
    NetworkRuntimePort for T
{
}
pub trait VolumeRuntimePort:
    VolumeInventoryPort + VolumeObservationPort + VolumeMutationPort
{
}
impl<T: VolumeInventoryPort + VolumeObservationPort + VolumeMutationPort + ?Sized> VolumeRuntimePort
    for T
{
}
pub trait NetworkVolumeInventoryPort: NetworkInventoryPort + VolumeInventoryPort {}
impl<T: NetworkInventoryPort + VolumeInventoryPort + ?Sized> NetworkVolumeInventoryPort for T {}
pub trait ContainerStreamsPort: logs::ContainerLogPort + terminal::ContainerTerminalPort {}
impl<T: logs::ContainerLogPort + terminal::ContainerTerminalPort + ?Sized> ContainerStreamsPort
    for T
{
}
pub trait PlatformRuntimeProvider: swarm_mutations::SwarmSnapshotPort + Send + Sync {
    fn swarm<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmRuntimePort + 'a>, RuntimeCapabilityError>>;
    fn image_mutations<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn ImageMutationRuntimePort + 'a>, RuntimeCapabilityError>>;
    fn pruning<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn prune::PlatformPrunePort + 'a>, RuntimeCapabilityError>>;
    fn networks<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkRuntimePort + 'a>, RuntimeCapabilityError>>;
    fn volumes<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn VolumeRuntimePort + 'a>, RuntimeCapabilityError>>;
    fn container_inspection<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Box<dyn containers::ContainerInspectionPort + 'a>, RuntimeCapabilityError>,
    >;
    fn images<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn images::ImageInspectionPort + 'a>, RuntimeCapabilityError>>;
    fn logs<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn logs::LogReadPort + 'a>, RuntimeCapabilityError>>;
    fn inventory<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkVolumeInventoryPort + 'a>, RuntimeCapabilityError>>;
    fn tasks<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmTaskRuntimePort + 'a>, RuntimeCapabilityError>>;
    fn streams(&self, platform: Uuid, node: Option<&str>) -> Box<dyn ContainerStreamsPort>;
}
