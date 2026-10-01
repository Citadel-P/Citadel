//! Image and prune transport dispatch; callers use feature-owned capability ports.
use super::runtime::Runtime;
use citadel_platforms::{
    ImageInventoryPort, RuntimeCapabilityError, RuntimeImageSummary,
    image_pull::{ImagePullPort, ImagePullStream},
    images::ImageDeletionPort,
    prune::{PlatformPrunePort, PrunePlatformOutcome, PruneResource},
};
use futures_util::future::BoxFuture;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

impl ImageInventoryPort for Runtime<'_> {
    fn list_images<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => ImageInventoryPort::list_images(*r, cancel),
            Runtime::Agent(r) => ImageInventoryPort::list_images(r, cancel),
            Runtime::Edge(r) => ImageInventoryPort::list_images(r, cancel),
        }
    }
}
impl ImagePullPort for Runtime<'_> {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => ImagePullPort::pull_image_stream(*r, image, auth, cancel),
            Runtime::Agent(r) => ImagePullPort::pull_image_stream(r, image, auth, cancel),
            Runtime::Edge(r) => ImagePullPort::pull_image_stream(r, image, auth, cancel),
        }
    }
}
impl ImageDeletionPort for Runtime<'_> {
    fn delete_image<'a>(
        &'a self,
        id: &'a str,
        force: bool,
        no_prune: bool,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BTreeMap<String, String>>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => ImageDeletionPort::delete_image(*r, id, force, no_prune, cancel),
            Runtime::Agent(r) => ImageDeletionPort::delete_image(r, id, force, no_prune, cancel),
            Runtime::Edge(r) => ImageDeletionPort::delete_image(r, id, force, no_prune, cancel),
        }
    }
}
impl PlatformPrunePort for Runtime<'_> {
    fn prune<'a>(
        &'a self,
        resource: PruneResource,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PrunePlatformOutcome, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => PlatformPrunePort::prune(*r, resource, cancel),
            Runtime::Agent(r) => PlatformPrunePort::prune(r, resource, cancel),
            Runtime::Edge(r) => PlatformPrunePort::prune(r, resource, cancel),
        }
    }
}
