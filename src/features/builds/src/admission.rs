use crate::{BuildClaim, BuildError};
use futures_util::future::BoxFuture;
use uuid::Uuid;

/// Authenticated executor identity, independent of Platform and Pool aliases.
#[derive(Debug, Clone)]
pub struct BuildExecutorCapacity {
    pub daemon_id: String,
    pub authority_id: Uuid,
    pub admission_generation: u64,
    pub build_slots: u32,
    pub heavy_slots: u32,
}

#[derive(Debug, PartialEq)]
pub enum BuildAdmission {
    Reserved,
    CapacityBusy,
    CleanupPending,
}

#[derive(Debug, Default)]
pub struct BuildExecutorOccupancy {
    pub authority_id: Option<Uuid>,
    pub reserved: u64,
    pub uncertain: u64,
}

pub trait BuildAdmissionStore: Send + Sync {
    fn executor_occupancy<'a>(
        &'a self,
        daemon_id: &'a str,
    ) -> BoxFuture<'a, Result<BuildExecutorOccupancy, BuildError>>;

    /// Record authenticated, same-authority cleanup evidence. Occupancy stays
    /// reserved until terminal persistence (or startup recovery) commits.
    fn record_executor_cleanup<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;

    /// Reserve a host before preparing Git input. No execution may be sent until
    /// dispatch_executor has atomically pinned the input and marked dispatch.
    fn reserve_preparation<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
    ) -> BoxFuture<'a, Result<BuildAdmission, BuildError>>;

    fn dispatch_executor<'a>(
        &'a self,
        claim: &'a BuildClaim,
        commit: &'a str,
    ) -> BoxFuture<'a, Result<(), BuildError>>;

    /// Persist ownership and the resolved input before sending an execution request.
    /// A repeated reservation is a conflict, not permission to dispatch twice.
    fn reserve_executor<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
        commit: &'a str,
    ) -> BoxFuture<'a, Result<BuildAdmission, BuildError>>;
}
