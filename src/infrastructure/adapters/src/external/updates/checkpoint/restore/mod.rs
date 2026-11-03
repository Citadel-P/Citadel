//! Isolated PostgreSQL restore rehearsal. Does not authorize replacement or resume.
mod evidence;
mod keys;
mod runtime;
mod sandbox;

pub(in crate::external::updates) use evidence::verified_checkpoint;

pub use runtime::{CheckpointRestoreBudget, RestoredUpdateCheckpoint, verify_update_checkpoint};

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod tests;
