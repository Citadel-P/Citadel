//! Each reconciler sees only its matching resource capability.
pub use crate::PlatformInfoPort as PlatformMetadataPort;
use crate::PlatformInfoPort;
pub use crate::{
    ContainerInventoryPort, ImageInventoryPort, NetworkInventoryPort, VolumeInventoryPort,
};
use crate::{
    RuntimeCapabilityError, RuntimeContainerSummary, RuntimeImageSummary, RuntimeNetworkSummary,
    RuntimePlatformInfo, RuntimeVolumeSummary,
};
#[cfg(test)]
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub struct ContainerReconciler;
impl ContainerReconciler {
    pub async fn collect<T: ContainerInventoryPort + ?Sized>(
        runtime: &T,
        cancel: &CancellationToken,
    ) -> Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError> {
        runtime.list_containers(cancel).await
    }
}

pub struct ImageReconciler;
impl ImageReconciler {
    pub async fn collect<T: ImageInventoryPort + ?Sized>(
        runtime: &T,
        cancel: &CancellationToken,
    ) -> Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError> {
        runtime.list_images(cancel).await
    }
}

pub struct PlatformReconciler;
impl PlatformReconciler {
    pub async fn collect<T: PlatformInfoPort + ?Sized>(
        runtime: &T,
        cancel: &CancellationToken,
    ) -> Result<RuntimePlatformInfo, RuntimeCapabilityError> {
        runtime.get_info(cancel).await
    }
}

pub struct NetworkReconciler;
impl NetworkReconciler {
    pub async fn collect<T: NetworkInventoryPort + ?Sized>(
        runtime: &T,
        cancel: &CancellationToken,
    ) -> Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError> {
        runtime.list_networks(cancel).await
    }
}

pub struct VolumeReconciler;
impl VolumeReconciler {
    pub async fn collect<T: VolumeInventoryPort + ?Sized>(
        runtime: &T,
        cancel: &CancellationToken,
    ) -> Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError> {
        runtime.list_volumes(cancel).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct ContainersOnly;
    impl ContainerInventoryPort for ContainersOnly {
        fn list_containers<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            Box::pin(async { Ok(vec![]) })
        }
    }
    struct ImagesOnly;
    impl ImageInventoryPort for ImagesOnly {
        fn list_images<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
            Box::pin(async { Ok(vec![]) })
        }
    }
    struct PlatformOnly;
    impl PlatformMetadataPort for PlatformOnly {
        fn get_info<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            Box::pin(async { Ok(RuntimePlatformInfo::default()) })
        }
    }
    struct NetworkOnly;
    impl NetworkInventoryPort for NetworkOnly {
        fn list_networks<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
            Box::pin(async { Ok(vec![]) })
        }
    }
    struct VolumeOnly;
    impl VolumeInventoryPort for VolumeOnly {
        fn list_volumes<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
            Box::pin(async { Ok(vec![]) })
        }
    }
    #[tokio::test]
    async fn scope_dispatch_accepts_only_the_selected_capability() {
        use crate::jobs::{ResourceCollector, collect_event_scope};
        let cancel = CancellationToken::new();
        for source in [
            ResourceCollector::Platform(&PlatformOnly),
            ResourceCollector::Containers(&ContainersOnly),
            ResourceCollector::Images(&ImagesOnly),
            ResourceCollector::Networks(&NetworkOnly),
            ResourceCollector::Volumes(&VolumeOnly),
        ] {
            collect_event_scope(source, uuid::Uuid::now_v7(), &cancel)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn reconcilers_accept_ports_without_any_unrelated_capability() {
        let cancel = CancellationToken::new();
        PlatformReconciler::collect(&PlatformOnly, &cancel)
            .await
            .unwrap();
        assert!(
            NetworkReconciler::collect(&NetworkOnly, &cancel)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            VolumeReconciler::collect(&VolumeOnly, &cancel)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            ContainerReconciler::collect(&ContainersOnly, &cancel)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            ImageReconciler::collect(&ImagesOnly, &cancel)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
