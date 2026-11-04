//! Instance update dependencies and startup composition.
mod checks;
#[cfg(target_os = "linux")]
mod completion;
#[cfg(target_os = "linux")]
mod source;
#[cfg(target_os = "linux")]
mod startup;

pub(super) use checks::build;
#[cfg(target_os = "linux")]
pub(crate) use completion::verify_completion;
#[cfg(target_os = "linux")]
pub(crate) use source::recover_source;
#[cfg(target_os = "linux")]
pub(crate) use startup::accept_target;

#[cfg(target_os = "linux")]
mod installation;
#[cfg(target_os = "linux")]
pub(super) use installation::build_installation;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(not(test))]
pub(crate) mod releases;
#[cfg(test)]
pub(crate) use fixtures as releases;
