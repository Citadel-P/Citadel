//! Bounded diagnostic persistence, independent of execution ownership.
mod maintenance;
mod reporter;
pub use maintenance::run_maintenance;
pub use reporter::run;

#[cfg(test)]
mod tests;
