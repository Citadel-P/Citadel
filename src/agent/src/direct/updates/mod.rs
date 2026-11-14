//! Standard Docker Agent replacement and target-local admission.
mod cleanup;
mod gate;
mod runtime;
pub(crate) use gate::Gate;
pub(super) use gate::admission;

#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
mod tests;
