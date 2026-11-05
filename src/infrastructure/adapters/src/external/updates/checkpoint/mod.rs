//! Prepare recovery artifacts under an existing helper-owned installation lock.
mod integrity;
mod preparation;
mod recovery;
mod storage;
pub(in crate::external::updates) use integrity::verify_checkpoint_integrity;
pub use recovery::restore_update_checkpoint;

pub use preparation::{
    CheckpointInstallerInput, PreparedUpdateCheckpoint, prepare_update_checkpoint,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod recovery_tests;
