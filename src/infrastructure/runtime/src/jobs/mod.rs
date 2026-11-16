//! Bounded process-local observations; never scheduling or execution authority.
mod catalog;
mod history;
mod measure;
mod model;
pub use measure::{observe, observe_work};
mod observation;
pub use catalog::JobCatalog;
pub use history::{DiagnosticCapture, JobEvent};
pub use model::*;
pub use observation::{JobHandle, JobIteration, scope_owned};
#[cfg(test)]
mod tests;
