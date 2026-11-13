//! Resolve and verify the registered installer inputs without ambient Compose settings.
mod limits;
mod profile;
pub(in crate::external::updates) use limits::verify_limits;
mod publication;
mod replacement;
mod resolution;

pub use replacement::CheckedCoreReplacement;
pub use resolution::{CheckedInstallerCompose, InstallerComposeInputs};

#[cfg(test)]
mod replacement_tests;
#[cfg(test)]
pub(super) mod tests;

mod database;
pub(in crate::external::updates) use database::{database_update, validate_database_update};
