mod alerts;
mod automation;
mod backups;
mod builds;
mod deployments;
mod git;
mod platforms;
mod stacks;
mod swarm_services;

pub use platforms::{WorkerDependencies, WorkerSettings, register};
