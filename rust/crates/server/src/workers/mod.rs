mod alerts;
mod automation;
mod backups;
mod build_consumers;
mod builds;
mod containers;
mod deployments;
pub mod edge;
mod git;
mod platforms;
mod stacks;
mod swarm_services;
mod volume_helpers;

pub use platforms::{WorkerDependencies, WorkerSettings, register};
