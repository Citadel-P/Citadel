//! Workload HTTP shapes, authorization and semantic-to-wire mapping.
mod capabilities;
mod handlers;
pub mod requests;
pub mod spec;
pub mod views;
pub(crate) use handlers::documented_routes;
pub use handlers::{StacksHttpState, StacksRealtimeNotifier, router};

mod tasks;
pub use tasks::TrackedStackTasks;
