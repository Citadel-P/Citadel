//! Workload copying from current data or existing backups. Persistence owns
//! durable claims and finalization; runtime work executes through storage ports.
pub mod error;
pub mod model;
pub mod planning;
pub mod repository;
pub mod service;
pub use error::CopyError;
pub use model::*;
pub use repository::*;
pub use service::*;

#[cfg(test)]
mod tests;
