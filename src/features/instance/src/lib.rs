//! Installation-wide configuration, policy and persisted administrator choices.
#![forbid(unsafe_code)]
mod commands;
pub mod environment;
mod error;
mod model;
mod repository;
mod service;
mod validation;

pub use commands::{PatchInstallationSettings, PatchInstanceConfiguration};
pub use error::InstanceError;
pub use model::{
    InstallationPolicy, InstallationSettings, InstanceConfiguration, InstanceOverrides, LogFormat,
};
pub use repository::InstanceRepository;
pub use service::InstanceService;

#[cfg(test)]
mod tests;
