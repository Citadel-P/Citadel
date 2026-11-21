//! Coverage models, persisted read ports and evaluation.
mod index;
pub use index::IndexedCoverage;
mod model;
pub use model::*;
#[cfg(test)]
mod tests;
