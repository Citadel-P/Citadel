//! Executable logging setup for admitted Core startup and auxiliary CLI commands.
use citadel_server::config::{LogFormat, environment, log_color_from_env, log_format_from_env};
use tracing_subscriber::EnvFilter;

pub(crate) fn init(verbosity: &str, format: LogFormat) -> Result<(), Box<dyn std::error::Error>> {
    install(EnvFilter::try_new(verbosity)?, format)
}

/// Maintenance and auxiliary commands cannot depend on the settings database.
pub(crate) fn init_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    install(
        EnvFilter::try_from_env(environment::RUST_LOG).unwrap_or_else(|_| EnvFilter::new("info")),
        log_format_from_env()?,
    )
}

fn install(filter: EnvFilter, format: LogFormat) -> Result<(), Box<dyn std::error::Error>> {
    let color = log_color_from_env()?;
    match format {
        LogFormat::Text => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(color)
            .with_target(false)
            .init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_ansi(false)
            .with_current_span(false)
            .init(),
    }
    Ok(())
}
