//! Read-only recovery-key availability; never generate, copy or print keys.
mod validation;

pub use validation::{CheckedRecoveryKeys, RecoveryKeySource, check_recovery_keys};

#[cfg(test)]
mod tests;
