//! Detached Docker helper for standard Agent installations.
mod configuration;
mod replacement;
pub(crate) use configuration::{replacement_config, validate_image};
pub use replacement::replace;

#[cfg(test)]
mod tests;
