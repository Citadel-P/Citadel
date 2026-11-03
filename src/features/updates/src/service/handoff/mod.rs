//! Shutdown authorization after external acknowledgement and a fresh owned drain.
mod coordinator;
pub use coordinator::UpdateHandoffService;

#[cfg(test)]
mod tests;
