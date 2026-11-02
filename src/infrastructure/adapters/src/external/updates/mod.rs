//! Private installation registration, update journals and bounded release reads.
#[cfg(target_os = "linux")]
mod registration;
#[cfg(target_os = "linux")]
pub use registration::RegisteredComposeInstallation;
#[cfg(target_os = "linux")]
mod compose;
#[cfg(target_os = "linux")]
mod files;
#[cfg(target_os = "linux")]
pub use compose::{CheckedInstallerCompose, InstallerComposeInputs};
#[cfg(target_os = "linux")]
mod bootstrap;
#[cfg(target_os = "linux")]
pub use bootstrap::register_compose_installation;
#[cfg(target_os = "linux")]
mod journal;
#[cfg(target_os = "linux")]
pub use journal::{InstallationLock, JournalDirectory};
#[cfg(target_os = "linux")]
mod startup;
#[cfg(target_os = "linux")]
pub use startup::check_startup_registration;

mod releases;
pub use releases::GithubReleaseSource;

mod artifacts;
pub use artifacts::GithubReleaseArtifacts;
