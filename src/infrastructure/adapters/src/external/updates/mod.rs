//! Private installation registration, update journals and bounded release reads.
#[cfg(target_os = "linux")]
mod checkpoint;
#[cfg(target_os = "linux")]
mod docker;
#[cfg(target_os = "linux")]
pub use checkpoint::{
    CheckpointInstallerInput, CheckpointRestoreBudget, PreparedUpdateCheckpoint,
    RestoredUpdateCheckpoint, prepare_update_checkpoint, verify_update_checkpoint,
};
#[cfg(target_os = "linux")]
mod registration;
#[cfg(target_os = "linux")]
pub use registration::RegisteredComposeInstallation;
#[cfg(target_os = "linux")]
mod compose;
#[cfg(target_os = "linux")]
mod files;
#[cfg(target_os = "linux")]
mod recovery_keys;
#[cfg(target_os = "linux")]
pub use compose::{CheckedCoreReplacement, CheckedInstallerCompose, InstallerComposeInputs};
#[cfg(target_os = "linux")]
pub use recovery_keys::{CheckedRecoveryKeys, RecoveryKeySource, check_recovery_keys};
#[cfg(target_os = "linux")]
mod bootstrap;
#[cfg(target_os = "linux")]
pub use bootstrap::{
    CheckedUpdateInstallation, CheckpointStorage, check_registered_compose_installation,
    register_compose_installation,
};
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

#[cfg(target_os = "linux")]
mod inspection;
#[cfg(target_os = "linux")]
pub use inspection::DockerReleaseInspection;

#[cfg(target_os = "linux")]
mod handoff;
#[cfg(target_os = "linux")]
pub use handoff::{
    DockerHelperHandoff, HelperLaunchIntent, LaunchedUpdateHelper, MAX_HANDOFF_REQUEST,
    TargetRecoveryKeys, TargetUpdateCompletion, TargetUpdateEvidence, TargetUpdateSession,
    UpdateHelperBudget, execute_helper_handoff, launch_update_helper, run_update_helper,
};
