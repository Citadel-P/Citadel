#![forbid(unsafe_code)]

mod catalog;
mod runner;
pub use catalog::AppliedMigration;

pub use runner::{MigrationError, MigrationOutcome, MigrationRunner};

pub const SCHEMA_SQL: &str = include_str!("schema/schema.sql");
pub const MIGRATION_MANIFEST_JSON: &str = include_str!("../generated/migrations.json");
