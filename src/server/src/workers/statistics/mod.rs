//! Bounded sampling ingress and independently supervised persistence writers.
mod budget;
mod ingress;
mod writer;
pub use ingress::{StatsIngress, register};

#[cfg(test)]
mod tests;
