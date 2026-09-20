//! Server-owned resource API representations and inbound adapters.
pub mod deployments;

pub mod stacks;

pub mod swarm_services;

pub mod builds;

pub mod backups;

pub mod git;

pub mod alerts;
pub mod automation;

pub mod vocabulary;

pub mod bindings;
pub(crate) mod catalog_query;
pub mod discovery;
pub(crate) mod metadata_patch;
pub mod registries;
pub(crate) mod resource_access;
pub mod tags;
