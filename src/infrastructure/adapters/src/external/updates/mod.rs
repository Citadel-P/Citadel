//! Private installation registration, update journals and bounded release reads.
#[cfg(target_os = "linux")]
mod checkpoint;
#[cfg(target_os = "linux")]
mod docker;
#[cfg(target_os = "linux")]
mod layout;
#[cfg(target_os = "linux")]
pub use checkpoint::{
    CheckpointInstallerInput, PreparedUpdateCheckpoint, prepare_update_checkpoint,
    restore_update_checkpoint,
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
    AUTOMATIC_STATE_MOUNT, AutomaticComposeSetup, CheckedUpdateInstallation, CheckpointStorage,
    check_registered_compose_installation, discover_automatic_setup, refresh_compose_installation,
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

#[cfg(target_os = "linux")]
mod database_status;
#[cfg(target_os = "linux")]
pub use database_status::DockerDatabaseInstallation;

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
mod host;
#[cfg(target_os = "linux")]
pub use handoff::{
    CompletedUpdateTarget, DockerHelperHandoff, FinalizeUpdateRequest, HelperLaunchIntent,
    LaunchedUpdateHelper, MAX_HANDOFF_REQUEST, TargetRecoveryKeys, TargetUpdateCompletion,
    TargetUpdateEvidence, TargetUpdateSession, UpdateHelperBudget, completed_update_target,
    execute_helper_handoff, finalize_update, launch_update_helper, run_update_helper,
};
#[cfg(target_os = "linux")]
pub use host::{
    AutomaticComposeRegistration, DockerUpdateInstallation, HostUpdateClient, HostUpdateRequest,
    HostUpdateResponse, execute_host_update,
};
