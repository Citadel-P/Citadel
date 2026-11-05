//! Detached Docker helper for standard Agent installations.
mod configuration;
mod replacement;
mod settlement;
pub(crate) use configuration::{replacement_config, validate_image};
pub use replacement::replace;
pub(crate) use settlement::{helper_name, settled};

#[cfg(test)]
mod tests;
