//! Process-local admission and cooperative draining; persistence owns maintenance authority.
mod gate;
pub use gate::{AdmissionError, AdmissionGate, AdmissionPermit};

#[cfg(test)]
mod tests;
