//! Persisted metadata observations and bounded background scan input.
use crate::{
    RuntimeCapabilityError,
    image_updates::{ImageTagUpdate, ImageUpdateRegistry},
};
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub struct ImageUpdateCandidate {
    pub id: Uuid,
    pub platform: Uuid,
    pub image: String,
    pub tags: Vec<String>,
    pub registry_id: Option<Uuid>,
}
#[derive(Clone)]
pub struct StoredImageUpdate {
    pub image: String,
    pub registry: Uuid,
    pub revision: String,
    pub update: ImageTagUpdate,
}
pub trait ImageUpdateStore: Send + Sync {
    fn candidates(
        &self,
        after: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageUpdateCandidate>, RuntimeCapabilityError>>;
    fn load<'a>(
        &'a self,
        platform: Uuid,
        images: &'a [String],
        registries: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StoredImageUpdate>, RuntimeCapabilityError>>;
    /// Fences older attempts and references no longer present in the inventory.
    /// When authorized, atomically associates a successful check's Registry with
    /// an unassociated image. Never replaces an existing association.
    fn save<'a>(
        &'a self,
        platform: Uuid,
        image: &'a str,
        registry: &'a ImageUpdateRegistry,
        result: &'a ImageTagUpdate,
        remember_registry: bool,
    ) -> BoxFuture<'a, Result<bool, RuntimeCapabilityError>>;
}
