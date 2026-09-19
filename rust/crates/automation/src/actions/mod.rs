mod commands;
mod model;
pub mod patch;
mod policy;
pub use commands::AutomationActionConfiguration;
pub use model::AutomationAction;
pub use patch::{UpdateAutomationActionInput, UpdateAutomationActionMetadata};
pub use policy::changes_paid_trigger;
