//! Supervised release checking and installation execution.
mod checker;
mod installation;
pub use checker::run;
pub use installation::run_installation;

mod agents;
pub use agents::run_agents;

#[cfg(target_os = "linux")]
mod automatic;
#[cfg(target_os = "linux")]
pub use automatic::run_automatic_setup;
