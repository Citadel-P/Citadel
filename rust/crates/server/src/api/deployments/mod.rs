//! Deployment inbound adapter: HTTP shapes and semantic-to-wire mapping.
mod adoption;
mod capabilities;
mod handlers;
pub mod requests;
pub mod views;
pub(crate) use handlers::documented_routes;
pub use handlers::{DeploymentsHttpState, DeploymentsRealtimeNotifier, router};

mod tasks;
pub use tasks::TrackedDeploymentTasks;

pub mod spec;

mod adoption_views;
