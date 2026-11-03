//! Isolated descriptor execution. Never mounts or replaces an installation.
mod docker;
mod probe;
mod runtime;

pub use runtime::DockerReleaseInspection;

#[cfg(test)]
mod tests;
