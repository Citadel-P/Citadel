//! Short-lived host namespace access for the installed Core.
mod command;
mod installation;
mod runtime;

pub use command::{HostUpdateRequest, HostUpdateResponse, execute_host_update};
pub use installation::DockerUpdateInstallation;
pub use runtime::HostUpdateClient;

#[cfg(test)]
mod tests;

mod automatic;
pub use automatic::AutomaticComposeRegistration;
