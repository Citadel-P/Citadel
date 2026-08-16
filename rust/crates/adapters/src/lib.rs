#![forbid(unsafe_code)]

pub mod docker;
mod postgres;

pub use postgres::PostgresAuthorizedPlatformReader;
