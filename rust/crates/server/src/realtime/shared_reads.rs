//! Raw committed rows shared for one published event only. Callers authorize
//! before reading and construct actor-specific views after reading.
use super::{PublishedRuntimeEvent, RealtimeReadError};
use citadel_platforms::{ContainerDetails, ImageDetails, PlatformDetails, PlatformReadService};
use citadel_runtime::runtime_metrics::RuntimeWork;
use tokio::sync::OnceCell;
use uuid::Uuid;

#[derive(Debug, Default)]
pub(crate) struct CommittedReads {
    platform: OnceCell<Option<PlatformDetails>>,
    containers: OnceCell<Vec<ContainerDetails>>,
    images: OnceCell<Vec<ImageDetails>>,
    pub(crate) networks: OnceCell<Vec<citadel_platforms::RuntimeNetworkSummary>>,
    pub(crate) volumes: OnceCell<Vec<citadel_platforms::RuntimeVolumeSummary>>,
    node_networks: OnceCell<
        Vec<citadel_platforms::NodeResourceProjection<citadel_platforms::RuntimeNetworkSummary>>,
    >,
    node_volumes: OnceCell<
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
shared_read!(platform, platform, get_platform, Option<PlatformDetails>);
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
