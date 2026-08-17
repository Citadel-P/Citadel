#![forbid(unsafe_code)]

pub mod activity_store;
pub mod agent;
pub mod container_stats_store;
pub mod crypto;
pub mod docker;
pub mod identity_store;
pub mod inventory_projection_store;
pub mod license;
pub mod mfa;
pub mod oidc_protocol;
pub mod oidc_store;
pub mod platform_read_store;
mod postgres;
pub mod postgres_runtime;
pub mod profile_store;
pub mod role_store;
pub mod service_account_store;
pub mod team_store;
pub mod user_store;

pub use postgres::PostgresAuthorizedPlatformReader;
