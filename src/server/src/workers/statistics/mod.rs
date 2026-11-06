//! Bounded sampling ingress and independently supervised persistence writers.
mod ingress;
mod writer;
pub use ingress::{StatsIngress, register};

#[cfg(test)]
mod tests;
