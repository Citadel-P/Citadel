//! Narrow reads used by background maintenance; no database types cross these ports.
use crate::RuntimeCapabilityError;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub struct UnmanagedContainer {
    pub id: Uuid,
    pub name: String,
    pub platform_name: String,
    pub platform_address: String,
}

pub trait ContainerMaintenanceReader: Send + Sync {
    fn pruning_candidates(&self) -> BoxFuture<'_, Result<Vec<Uuid>, RuntimeCapabilityError>>;
    fn unmanaged_container<'a>(
        &'a self,
        platform: Uuid,
        container: &'a str,
        node: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<UnmanagedContainer>, RuntimeCapabilityError>>;
}

pub trait EdgeHealthReader: Send + Sync {
    /// Only nonrevoked manager bindings participate; unbound/node-only platforms are absent.
    fn health<'a>(
        &'a self,
        platforms: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<(Uuid, bool)>, RuntimeCapabilityError>>;
}
