//! Durable Docker-helper handoff and shutdown, separate from release discovery.
mod checkpoint;
mod command;
mod completion;
mod driver;
mod evidence;
mod keys;
mod launcher;
mod preparation;
mod replacement;
mod runtime;
mod session;
mod state;
mod target;
mod target_runtime;
mod verification;

pub use command::{MAX_HANDOFF_REQUEST, execute_helper_handoff};
pub use completion::TargetUpdateCompletion;
pub use driver::{UpdateHelperBudget, run_update_helper};
pub use evidence::HelperLaunchIntent;
pub use keys::TargetRecoveryKeys;
pub use launcher::{LaunchedUpdateHelper, launch_update_helper};
pub use runtime::DockerHelperHandoff;
use session::OwnedUpdateHelper;
pub(super) use state::ExecutionJournal;
pub use target::TargetUpdateEvidence;
pub use target_runtime::TargetUpdateSession;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod replacement_tests;

#[cfg(test)]
mod target_tests;
