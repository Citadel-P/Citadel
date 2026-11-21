//! Supervised release checking and installation execution.
pub(crate) mod checker;
mod installation;
pub use installation::run_installation;

mod agents;
pub use agents::run_agents;

#[cfg(target_os = "linux")]
mod automatic;
#[cfg(target_os = "linux")]
pub use automatic::run_automatic_setup;
