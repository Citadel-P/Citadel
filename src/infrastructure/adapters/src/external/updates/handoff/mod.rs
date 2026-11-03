//! Durable Docker-helper handoff and shutdown, separate from release discovery.
mod command;
mod evidence;
mod launcher;
mod runtime;
mod session;
mod verification;

pub use command::{MAX_HANDOFF_REQUEST, execute_helper_handoff};
pub use evidence::{HelperLaunchIntent, bind_helper_launch, prepare_helper_launch};
pub use launcher::{LaunchedUpdateHelper, launch_update_helper};
pub use runtime::DockerHelperHandoff;
pub use session::OwnedUpdateHelper;

#[cfg(test)]
mod tests;
