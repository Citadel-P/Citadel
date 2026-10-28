mod claims;
pub(crate) mod logs;
mod model;
pub(crate) mod progress;
pub use claims::{ScriptClaimResult, ScriptRunClaim, ScriptRunResult};
pub use logs::{code_hash, redact_logs};
pub use model::ScriptRun;
pub use progress::{ScriptProgress, ScriptProgressError};

/// Draft code authorized against this exact saved Script configuration.
/// The repository verifies the version while locking and snapshotting the run.
#[derive(Debug, Clone)]
pub struct ScriptDraft {
    pub code: String,
    pub expected_row_version: i64,
}
