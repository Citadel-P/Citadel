//! Administrator-only instance and Agent updates.
mod agents;
mod instance;
pub(crate) use instance::documented_routes;
pub use instance::{UpdateHttpState, router};
