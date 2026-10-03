use super::runtime::PlatformRuntimeRouter;
use citadel_platforms::{runtime_provider::*, *};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
impl PlatformRuntimeProvider for PlatformRuntimeRouter {
    fn swarm<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmRuntimePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, None, false, cancel).await?)
                as Box<dyn SwarmRuntimePort + 'a>)
        })
    }
    fn image_mutations<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn ImageMutationRuntimePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, None, false, cancel).await?)
                as Box<dyn ImageMutationRuntimePort + 'a>)
        })
    }
    fn pruning<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn prune::PlatformPrunePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, None, false, cancel).await?)
                as Box<dyn prune::PlatformPrunePort + 'a>)
        })
    }
    fn networks<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkRuntimePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, node, false, cancel).await?)
                as Box<dyn NetworkRuntimePort + 'a>)
        })
    }
    fn volumes<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn VolumeRuntimePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, node, false, cancel).await?)
                as Box<dyn VolumeRuntimePort + 'a>)
        })
    }
    fn container_inspection<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Box<dyn containers::ContainerInspectionPort + 'a>, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, node, false, cancel).await?)
                as Box<dyn containers::ContainerInspectionPort + 'a>)
        })
    }
    fn images<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn images::ImageInspectionPort + 'a>, RuntimeCapabilityError>>
    {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, node, false, cancel).await?)
                as Box<dyn images::ImageInspectionPort + 'a>)
        })
    }
    fn logs<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn logs::LogReadPort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, node, false, cancel).await?)
                as Box<dyn logs::LogReadPort + 'a>)
        })
    }
    fn inventory<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkVolumeInventoryPort + 'a>, RuntimeCapabilityError>>
    {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, None, false, cancel).await?)
                as Box<dyn NetworkVolumeInventoryPort + 'a>)
        })
    }
    fn tasks<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmTaskRuntimePort + 'a>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(Box::new(self.resolve(platform, None, false, cancel).await?)
                as Box<dyn SwarmTaskRuntimePort + 'a>)
        })
    }
    fn streams(&self, platform: Uuid, node: Option<&str>) -> Box<dyn ContainerStreamsPort> {
        Box::new(self.streams(platform, node))
    }
}
