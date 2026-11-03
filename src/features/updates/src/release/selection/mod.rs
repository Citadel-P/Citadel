mod policy;
pub use policy::{Release, available_update, development_update, is_development};

#[cfg(test)]
mod tests;
