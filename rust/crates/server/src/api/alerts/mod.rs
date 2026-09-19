mod handlers;
mod requests;
pub mod views;
pub(crate) use handlers::documented_routes;
pub use handlers::{AlertsHttpState, router};
