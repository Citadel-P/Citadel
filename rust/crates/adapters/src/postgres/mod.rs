mod platform_reader;
pub use platform_reader::PostgresAuthorizedPlatformReader;
pub mod deployments;

pub mod stacks;

pub mod swarm_services;

pub mod backups;
pub mod builds;
pub mod git;

pub mod alerts;
pub mod automation;

pub mod platform_classification;

pub(crate) mod authorization;
pub mod bindings;
pub mod registries;
pub mod tags;

pub mod platforms;
