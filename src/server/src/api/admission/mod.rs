//! Request admission for a coordinated maintenance drain.
mod middleware;
pub use middleware::admit;

#[cfg(test)]
mod tests;
