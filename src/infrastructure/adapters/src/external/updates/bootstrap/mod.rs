//! Registration and live installation preflight without replacement or database writes.
mod checkpoint;
mod executor;
mod inspection;
mod observation;
mod preflight;
mod registration;
mod replacement;
pub(super) use replacement::{ObservedReplacement, observe_replacement};

pub use checkpoint::CheckpointStorage;
pub use preflight::{CheckedUpdateInstallation, check_registered_compose_installation};
pub use registration::{refresh_compose_installation, register_compose_installation};

#[cfg(test)]
mod tests;

mod handoff;
pub(super) use executor::validate_executor_mounts;
pub(super) use handoff::{verify_handoff_source, verify_stopped_source};

#[cfg(test)]
pub(super) mod fixtures;

pub(super) use executor::validate_executor;
