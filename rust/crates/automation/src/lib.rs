#![forbid(unsafe_code)]
pub mod actions;
mod error;
pub mod jobs;
mod options;
pub mod permissions;
mod repository;
pub mod runs;
mod runtime;
mod service;
mod tasks;
pub use actions::{
    AutomationAction, AutomationActionConfiguration, UpdateAutomationActionInput,
    UpdateAutomationActionMetadata, changes_paid_trigger,
};
pub use error::AutomationError;
pub use options::AutomationOptions;
pub use repository::AutomationRepository;
pub use runs::{
    AutomationProgress, AutomationProgressError, AutomationRun, AutomationRunClaim,
    AutomationRunResult, code_hash, redact_logs,
};
pub use runtime::{AutomationEntitlements, AutomationRunTokenIssuer, AutomationRuntimeConfig};
pub use service::AutomationService;
pub use tasks::AutomationTaskSpawner;
#[cfg(test)]
mod tests;
