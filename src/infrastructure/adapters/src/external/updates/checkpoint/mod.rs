//! Prepare recovery artifacts under an existing helper-owned installation lock.
mod preparation;
mod restore;
mod storage;
pub(in crate::external::updates) use restore::verified_checkpoint;
pub use restore::{CheckpointRestoreBudget, RestoredUpdateCheckpoint, verify_update_checkpoint};

pub use preparation::{
    CheckpointInstallerInput, PreparedUpdateCheckpoint, prepare_update_checkpoint,
};

#[cfg(test)]
mod tests;
