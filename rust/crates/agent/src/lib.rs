#![forbid(unsafe_code)]

pub mod app;
pub mod cli;
mod composition;
pub mod config;

mod direct;
mod edge;

pub const VERSION: &str = env!("CITADEL_BUILD_VERSION");
pub const INFORMATIONAL_VERSION: &str = env!("CITADEL_BUILD_INFORMATIONAL_VERSION");
