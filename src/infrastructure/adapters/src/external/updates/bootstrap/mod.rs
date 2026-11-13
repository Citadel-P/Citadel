//! Registration and live installation preflight without replacement or database writes.
mod checkpoint;
mod executor;
mod inspection;
mod observation;
mod preflight;
mod registration;
mod replacement;
pub(super) use replacement::{ObservedReplacement, observe_accepted_target, observe_replacement};

pub use checkpoint::CheckpointStorage;
pub use preflight::{CheckedUpdateInstallation, check_registered_compose_installation};
pub use registration::{refresh_compose_installation, register_compose_installation};

#[cfg(test)]
mod tests;

mod handoff;
pub(super) use executor::validate_executor_mounts;
pub(super) use handoff::{verify_handoff_source, verify_recoverable_source, verify_stopped_source};

#[cfg(test)]
pub(super) mod fixtures;

pub(super) use executor::validate_executor;

mod automatic;
pub use automatic::{AUTOMATIC_STATE_MOUNT, AutomaticComposeSetup, discover_automatic_setup};
pub(super) use automatic::{complete_automatic_setup, prepare_automatic_setup};
#[cfg(test)]
mod automatic_tests;
