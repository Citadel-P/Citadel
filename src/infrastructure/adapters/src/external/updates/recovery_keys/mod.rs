//! Read-only recovery-key availability; never generate, copy or print keys.
mod validation;

pub use validation::{CheckedRecoveryKeys, RecoveryKeySource, check_recovery_keys};
pub(super) use validation::{check_packaged_recovery_keys, read_key};

#[cfg(test)]
mod tests;
