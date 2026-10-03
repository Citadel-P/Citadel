use crate::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub trait RegistryConnectionChecker: Send + Sync {
    fn check<'a>(&'a self, configuration: &'a Value) -> BoxFuture<'a, Result<(), RegistryError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryMutationKind {
    Update,
    Metadata,
    Rename,
}
pub trait RegistryRepository: Send + Sync {
    /// Resolve only identity; callers authorize before loading credentials.
    fn find_id_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, RegistryError>>;

    fn list_registries<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<RegistryDetails>, RegistryError>>;

    fn get_registry<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>>;

    fn create_registry<'a>(
        &'a self,
        actor_id: ActorId,
        registry: &'a NewRegistry,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>>;

    fn update_registry<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a RegistryPatch,
        kind: RegistryMutationKind,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>>;

    fn delete_registries<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), RegistryError>>;
}
