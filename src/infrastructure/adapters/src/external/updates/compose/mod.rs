//! Resolve and verify the registered installer inputs without ambient Compose settings.
mod profile;
mod publication;
mod replacement;
mod resolution;

pub use replacement::CheckedCoreReplacement;
pub use resolution::{CheckedInstallerCompose, InstallerComposeInputs};

#[cfg(test)]
mod replacement_tests;
#[cfg(test)]
pub(super) mod tests;
