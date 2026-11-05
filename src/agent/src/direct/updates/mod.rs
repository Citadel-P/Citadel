//! Standard Docker Agent replacement and target-local admission.
mod gate;
mod runtime;
pub(crate) use gate::Gate;
pub(super) use gate::admission;

#[cfg(test)]
mod tests;
