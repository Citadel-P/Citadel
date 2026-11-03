//! Isolated PostgreSQL restore rehearsal. Does not authorize replacement or resume.
mod evidence;
mod runtime;
mod sandbox;

pub use runtime::{CheckpointRestoreBudget, RestoredUpdateCheckpoint, verify_update_checkpoint};

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod tests;
