//! Installation identity, reviewed plans, operation state and recovery evidence.
mod identity;
mod journal;
mod operations;
mod plan;
#[cfg(test)]
mod tests;

pub use identity::{CoreIdentity, InstallationKind};
pub use journal::{CompletionReceipt, UPDATE_PROTOCOL, UpdateJournal};
pub use operations::{UpdateOperation, UpdatePhase};
pub use plan::UpdatePlan;
