//! Authorized, scoped coverage reads from persisted observations.
pub mod changes;
mod inventory;
mod reader;
pub use reader::{PostgresVolumeCoverageReader, volume_coverage};
