use crate::RuntimeCapabilityError;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PruneResource {
    All,
    Volume,
    Network,
    Image,
    Build,
}
impl PruneResource {
    pub const fn code(self) -> i32 {
        match self {
            Self::All => 1,
            Self::Volume => 2,
            Self::Network => 3,
            Self::Image => 4,
            Self::Build => 5,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrunePlatformInput {
    pub resource: PruneResource,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrunePlatformView {
    pub resource: PruneResource,
    pub space_reclaimed: i64,
    pub volumes_deleted: Vec<String>,
    pub networks_deleted: Vec<String>,
    pub images_deleted: Vec<String>,
    pub build_cache_deleted: Vec<String>,
}
impl PrunePlatformView {
    pub fn empty(resource: PruneResource) -> Self {
        Self {
            resource,
            space_reclaimed: 0,
            volumes_deleted: vec![],
            networks_deleted: vec![],
            images_deleted: vec![],
            build_cache_deleted: vec![],
        }
    }
}
pub trait PlatformPrunePort: Send + Sync {
    fn prune<'a>(
        &'a self,
        resource: PruneResource,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PrunePlatformView, RuntimeCapabilityError>>;
}
