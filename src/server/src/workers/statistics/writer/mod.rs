//! Bounded telemetry buffering with independent per-platform retries.
mod ingress;
mod runtime;
use ingress::Envelope;
pub use ingress::{Ingress, WriterSettings, channel};
pub use runtime::run;
#[cfg(test)]
mod tests;
