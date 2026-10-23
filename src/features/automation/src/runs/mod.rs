mod claims;
pub(crate) mod logs;
mod model;
pub(crate) mod progress;
pub use claims::{AutomationRunClaim, AutomationRunResult};
pub use logs::{code_hash, redact_logs};
pub use model::AutomationRun;
pub use progress::{AutomationProgress, AutomationProgressError};

/// Draft code authorized against this exact saved Action configuration.
/// The repository verifies the version while locking and snapshotting the run.
#[derive(Debug, Clone)]
pub struct AutomationDraft {
    pub code: String,
    pub expected_row_version: i64,
}
