//! Shared PostgreSQL recovery bundle construction and restoration.
mod builder;
mod files;
mod postgres;
mod recovery;
mod versions;

pub use builder::{PgDumpSystemBackupBuilder, RecoveryAssets};
pub(crate) use recovery::inspect_recovery_bundle;
pub use recovery::{CitadelSystemRestoreOptions, restore_citadel_system};
pub(crate) use versions::verify_backup_tools;

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
