//! Instance update dependencies and startup composition.
mod checks;
#[cfg(target_os = "linux")]
mod completion;
#[cfg(target_os = "linux")]
mod startup;

pub(super) use checks::build;
#[cfg(target_os = "linux")]
pub(crate) use completion::verify_completion;
#[cfg(target_os = "linux")]
pub(crate) use startup::accept_target;

#[cfg(target_os = "linux")]
mod installation;
#[cfg(target_os = "linux")]
pub(super) use installation::build_installation;
