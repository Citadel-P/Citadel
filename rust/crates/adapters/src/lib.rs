#![forbid(unsafe_code)]

pub mod agent;
pub mod docker;
mod postgres;

pub use postgres::PostgresAuthorizedPlatformReader;
