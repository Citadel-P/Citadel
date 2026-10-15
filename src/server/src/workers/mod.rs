pub(crate) mod alerts;
pub(crate) mod automation;
pub(crate) mod backup_coverage;
pub(crate) mod backups;
pub(crate) mod build_consumers;
pub(crate) mod builds;
pub(crate) mod container_batch;
pub(crate) mod containers;
pub(crate) mod deployments;
pub mod edge;
pub(crate) mod git;
pub(crate) mod platforms;
pub(crate) mod stacks;
pub(crate) mod swarm_services;
pub(crate) mod volume_helpers;

pub use crate::composition::workers::{WorkerDependencies, WorkerSettings, register};

pub(crate) mod disk;

#[cfg(test)]
pub(crate) mod disk_tests;

pub(crate) mod connection_events;
pub(crate) mod maintenance;
pub(crate) mod notifications;
pub(crate) mod recovery;
pub(crate) mod schedule;
pub(crate) mod unmanaged;

pub(crate) mod pruning;

#[cfg(test)]
use test_support::listener;

pub mod image_scanning;

pub(crate) mod stats_alerts;

#[cfg(test)]
pub(crate) mod runtime_persistence_tests;

pub mod statistics;

#[cfg(test)]
pub(crate) mod test_support;

pub mod resource_copies;
